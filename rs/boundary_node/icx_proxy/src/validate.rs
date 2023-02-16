use std::{borrow::Cow, io::Read};

use candid::Principal;
use flate2::read::{DeflateDecoder, GzDecoder};
use hyper::Uri;
use ic_agent::{
    hash_tree::{HashTree, LookupResult},
    lookup_value, Agent, AgentError, Certificate,
};
use sha2::{Digest, Sha256};
use tracing::trace;

use crate::headers::HeadersData;

// The limit of a buffer we should decompress ~10mb.
const MAX_CHUNK_SIZE_TO_DECOMPRESS: usize = 1024;
const MAX_CHUNKS_TO_DECOMPRESS: u64 = 10_240;

pub trait Validate: Sync + Send {
    fn validate(
        &self,
        required: bool,
        headers_data: &HeadersData,
        canister_id: &Principal,
        agent: &Agent,
        uri: &Uri,
        response_body: &[u8],
    ) -> Result<(), Cow<'static, str>>;
}

#[derive(Clone)]
pub struct Validator {}

impl Validator {
    pub fn new() -> Self {
        Self {}
    }
}

impl Validate for Validator {
    fn validate(
        &self,
        required: bool,
        headers_data: &HeadersData,
        canister_id: &Principal,
        agent: &Agent,
        uri: &Uri,
        response_body: &[u8],
    ) -> Result<(), Cow<'static, str>> {
        let decoded_body = decode_body(response_body, headers_data.encoding.clone())
            .ok_or("Body could not be decoded")?;
        let body_sha = hash_body(response_body);
        let decoded_body_sha = hash_body(&decoded_body);

        let cert = headers_data.certificate.as_ref();
        let tree = headers_data.tree.as_ref();
        let body_valid = match (required, cert, tree) {
            // TODO: Remove this (FOLLOW-483)
            // Canisters don't have to provide certified variables
            // This should change in the future, grandfathering in current implementations
            (false, None, None) => return Ok(()),

            (_, Some(Ok(certificate)), Some(Ok(tree))) => {
                // first try to validate the body with the decoded body's hash
                match validate_body(
                    Certificates { certificate, tree },
                    canister_id,
                    agent,
                    uri,
                    &decoded_body_sha,
                ) {
                    Ok(true) => true,
                    // if that fails, try to validate the body using the original body's hash
                    Ok(false) | Err(_) => validate_body(
                        Certificates { certificate, tree },
                        canister_id,
                        agent,
                        uri,
                        &body_sha,
                    )
                    .map_err(|e| format!("Certificate validation failed: {e}"))?,
                }
            }

            (true, _, _) => return Err("Response verification required but not provided".into()),
            (_, Some(_), _) => {
                return Err("`Ic-Certificate` response header missing `tree` field".into())
            }
            (_, _, Some(_)) => {
                return Err("`Ic-Certificate` response header missing `certificate` field".into())
            }
        };

        if cfg!(feature = "skip_body_verification") || body_valid {
            return Ok(());
        }
        Err("Body does not pass verification".into())
    }
}

struct Certificates<'a> {
    certificate: &'a Vec<u8>,
    tree: &'a Vec<u8>,
}

fn decode_body(body: &[u8], encoding: Option<String>) -> Option<Vec<u8>> {
    match encoding.as_deref() {
        Some("gzip") => body_from_decoder(GzDecoder::new(body)),
        Some("deflate") => body_from_decoder(DeflateDecoder::new(body)),
        _ => Some(body.to_vec()),
    }
}

fn body_from_decoder<D: Read>(mut decoder: D) -> Option<Vec<u8>> {
    let mut decoded = Vec::new();
    let mut buffer = [0u8; MAX_CHUNK_SIZE_TO_DECOMPRESS];

    for _ in 0..MAX_CHUNKS_TO_DECOMPRESS {
        let bytes = decoder.read(&mut buffer).ok()?;

        if bytes == 0 {
            return Some(decoded);
        }

        decoded.extend_from_slice(&buffer[..bytes]);
    }

    if decoder.bytes().next().is_some() {
        return None;
    }

    Some(decoded)
}

fn hash_body(body: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(body);
    hasher.finalize().into()
}

fn validate_body(
    certificates: Certificates,
    canister_id: &Principal,
    agent: &Agent,
    uri: &Uri,
    body_sha: &[u8; 32],
) -> anyhow::Result<bool> {
    let cert: Certificate =
        serde_cbor::from_slice(certificates.certificate).map_err(AgentError::InvalidCborData)?;
    let tree: HashTree =
        serde_cbor::from_slice(certificates.tree).map_err(AgentError::InvalidCborData)?;

    if let Err(e) = agent.verify(&cert, *canister_id) {
        trace!(">> certificate failed verification: {}", e);
        return Ok(false);
    }

    let certified_data_path = vec![
        "canister".into(),
        canister_id.into(),
        "certified_data".into(),
    ];
    let witness = match lookup_value(&cert, certified_data_path) {
        Ok(witness) => witness,
        Err(e) => {
            trace!(
                ">> Could not find certified data for this canister in the certificate: {}",
                e
            );
            return Ok(false);
        }
    };
    let digest = tree.digest();

    if witness != digest {
        trace!(
            ">> witness ({}) did not match digest ({})",
            hex::encode(witness),
            hex::encode(digest)
        );

        return Ok(false);
    }

    let path = ["http_assets".into(), uri.path().into()];
    let tree_sha = match tree.lookup_path(&path) {
        LookupResult::Found(v) => v,
        _ => match tree.lookup_path(&["http_assets".into(), "/index.html".into()]) {
            LookupResult::Found(v) => v,
            _ => {
                trace!(
                    ">> Invalid Tree in the header. Does not contain path {:?}",
                    path
                );
                return Ok(false);
            }
        },
    };

    Ok(body_sha == tree_sha)
}

#[cfg(test)]
mod tests {
    use candid::Principal;
    use ic_agent::{
        agent::http_transport::{
            hyper::{Body, Uri},
            HyperReplicaV2Transport,
        },
        Agent,
    };

    use crate::{
        headers::HeadersData,
        validate::{Validate, Validator},
    };

    #[test]
    fn validate_nop() {
        let headers = HeadersData {
            certificate: None,
            encoding: None,
            tree: None,
        };

        let canister_id = Principal::from_text("wwc2m-2qaaa-aaaac-qaaaa-cai").unwrap();
        let uri = Uri::from_static("http://www.example.com");
        let transport = HyperReplicaV2Transport::<Body>::create(uri.clone()).unwrap();
        let agent = Agent::builder().with_transport(transport).build().unwrap();
        let body = vec![];

        let validator = Validator::new();

        let out = validator.validate(false, &headers, &canister_id, &agent, &uri, &body);

        assert_eq!(out, Ok(()));
    }
}
