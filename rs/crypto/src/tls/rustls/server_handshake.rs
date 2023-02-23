use crate::tls::rustls::cert_resolver::StaticCertResolver;
use crate::tls::rustls::csp_server_signing_key::CspServerEd25519SigningKey;
use crate::tls::rustls::node_cert_verifier::NodeClientCertVerifier;
use crate::tls::rustls::{certified_key, RustlsTlsStream};
use crate::tls::{
    node_id_from_cert_subject_common_name, tls_cert_from_registry, TlsCertFromRegistryError,
};
use ic_crypto_internal_csp::api::CspTlsHandshakeSignerProvider;
use ic_crypto_internal_csp::key_id::KeyId;
use ic_crypto_tls_interfaces::{
    AllowedClients, AuthenticatedPeer, TlsPublicKeyCert, TlsServerHandshakeError, TlsStream,
};
use ic_interfaces_registry::RegistryClient;
use ic_types::{NodeId, RegistryVersion};
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_rustls::rustls::ciphersuite::{TLS13_AES_128_GCM_SHA256, TLS13_AES_256_GCM_SHA384};
use tokio_rustls::rustls::sign::CertifiedKey;
use tokio_rustls::rustls::{
    ClientCertVerifier, NoClientAuth, ProtocolVersion, ResolvesServerCert, ServerConfig, Session,
    SignatureScheme,
};
use tokio_rustls::TlsAcceptor;

pub async fn perform_tls_server_handshake<P: CspTlsHandshakeSignerProvider>(
    signer_provider: &P,
    self_node_id: NodeId,
    registry_client: Arc<dyn RegistryClient>,
    tcp_stream: TcpStream,
    allowed_clients: AllowedClients,
    registry_version: RegistryVersion,
) -> Result<(Box<dyn TlsStream>, AuthenticatedPeer), TlsServerHandshakeError> {
    let self_tls_cert =
        tls_cert_from_registry(registry_client.as_ref(), self_node_id, registry_version)?;
    let self_tls_cert_key_id = KeyId::try_from(&self_tls_cert).map_err(|error| {
        TlsServerHandshakeError::MalformedSelfCertificate {
            internal_error: format!("Cannot instantiate KeyId: {:?}", error),
        }
    })?;
    let client_cert_verifier = NodeClientCertVerifier::new_with_mandatory_client_auth(
        allowed_clients.nodes().clone(),
        registry_client,
        registry_version,
    );
    let ed25519_signing_key =
        CspServerEd25519SigningKey::new(self_tls_cert_key_id, signer_provider.handshake_signer());
    let config = server_config_with_tls13_and_aes_ciphersuites_and_ed25519_signing_key(
        Arc::new(client_cert_verifier),
        self_tls_cert,
        ed25519_signing_key,
    );

    let rustls_stream = accept_connection(tcp_stream, config).await?;

    let client_cert_from_handshake = single_client_cert_from_handshake(&rustls_stream)?;
    let authenticated_peer = node_id_from_cert_subject_common_name(&client_cert_from_handshake)?;
    let tls_stream = RustlsTlsStream::new(tokio_rustls::TlsStream::from(rustls_stream));

    Ok((
        Box::new(tls_stream),
        AuthenticatedPeer::Node(authenticated_peer),
    ))
}

pub async fn perform_tls_server_handshake_without_client_auth<P: CspTlsHandshakeSignerProvider>(
    signer_provider: &P,
    self_node_id: NodeId,
    registry_client: &dyn RegistryClient,
    tcp_stream: TcpStream,
    registry_version: RegistryVersion,
) -> Result<Box<dyn TlsStream>, TlsServerHandshakeError> {
    let self_tls_cert = tls_cert_from_registry(registry_client, self_node_id, registry_version)?;
    let self_tls_cert_key_id = KeyId::try_from(&self_tls_cert).map_err(|error| {
        TlsServerHandshakeError::MalformedSelfCertificate {
            internal_error: format!("Cannot instantiate KeyId: {:?}", error),
        }
    })?;
    let ed25519_signing_key =
        CspServerEd25519SigningKey::new(self_tls_cert_key_id, signer_provider.handshake_signer());
    let config = server_config_with_tls13_and_aes_ciphersuites_and_ed25519_signing_key(
        NoClientAuth::new(),
        self_tls_cert,
        ed25519_signing_key,
    );

    let rustls_stream = accept_connection(tcp_stream, config).await?;

    Ok(Box::new(RustlsTlsStream::new(
        tokio_rustls::TlsStream::from(rustls_stream),
    )))
}

fn server_config_with_tls13_and_aes_ciphersuites_and_ed25519_signing_key(
    client_cert_verifier: Arc<dyn ClientCertVerifier>,
    self_tls_cert: TlsPublicKeyCert,
    ed25519_signing_key: CspServerEd25519SigningKey,
) -> ServerConfig {
    let mut config = ServerConfig::new(client_cert_verifier);
    config.versions = vec![ProtocolVersion::TLSv1_3];
    config.ciphersuites = vec![&TLS13_AES_256_GCM_SHA384, &TLS13_AES_128_GCM_SHA256];

    config.cert_resolver = static_cert_resolver(
        certified_key(self_tls_cert, ed25519_signing_key),
        SignatureScheme::ED25519,
    );
    config
}

async fn accept_connection(
    tcp_stream: TcpStream,
    config: ServerConfig,
) -> Result<tokio_rustls::server::TlsStream<TcpStream>, TlsServerHandshakeError> {
    TlsAcceptor::from(Arc::new(config))
        .accept(tcp_stream)
        .await
        .map_err(|e| TlsServerHandshakeError::HandshakeError {
            internal_error: format!("{}", e),
        })
}

fn static_cert_resolver(key: CertifiedKey, scheme: SignatureScheme) -> Arc<dyn ResolvesServerCert> {
    Arc::new(StaticCertResolver::new(key, scheme).expect(
        "Failed to create the static cert resolver because the signing key referenced \
        in the certified key is incompatible with the signature scheme. This is an implementation error.",
    ))
}

fn single_client_cert_from_handshake(
    tls_stream: &tokio_rustls::server::TlsStream<TcpStream>,
) -> Result<TlsPublicKeyCert, TlsServerHandshakeError> {
    let peer_certs = tls_stream.get_ref().1.get_peer_certificates().ok_or(
        TlsServerHandshakeError::HandshakeError {
            internal_error: "missing peer certificates in session".to_string(),
        },
    )?;
    if peer_certs.len() > 1 {
        return Err(TlsServerHandshakeError::HandshakeError {
            internal_error: "peer sent more than one certificate, but expected only a single one"
                .to_string(),
        });
    }
    let end_entity = peer_certs
        .first()
        .ok_or(TlsServerHandshakeError::HandshakeError {
            internal_error:
                "peer certificate chain is empty, but expected it to contain a single certificate"
                    .to_string(),
        })?;
    TlsPublicKeyCert::new_from_der(end_entity.0.clone()).map_err(|e| {
        TlsServerHandshakeError::HandshakeError {
            internal_error: format!(
                "failed to create TlsPublicKeyCert from DER: {}",
                e.internal_error
            ),
        }
    })
}

impl From<TlsCertFromRegistryError> for TlsServerHandshakeError {
    fn from(registry_error: TlsCertFromRegistryError) -> Self {
        match registry_error {
            TlsCertFromRegistryError::RegistryError(e) => TlsServerHandshakeError::RegistryError(e),
            TlsCertFromRegistryError::CertificateNotInRegistry {
                node_id,
                registry_version,
            } => TlsServerHandshakeError::CertificateNotInRegistry {
                node_id,
                registry_version,
            },
            TlsCertFromRegistryError::CertificateMalformed { internal_error } => {
                TlsServerHandshakeError::MalformedSelfCertificate { internal_error }
            }
        }
    }
}
