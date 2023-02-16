mod errors;

pub use errors::*;

use ic_protobuf::registry::crypto::v1::PublicKey as PublicKeyProto;
use ic_types::crypto::{CryptoResult, CurrentNodePublicKeys};
use ic_types::registry::RegistryClientError;
use ic_types::RegistryVersion;

/// Methods for checking and retrieving key material.
pub trait KeyManager {
    /// Checks whether this crypto component is properly set up and in sync with the registry. As
    /// part of the check, the number of public keys in the registry, as well as the corresponding
    /// local public and secret keys, are counted, and metrics observations are made.
    ///
    /// This is done by ensuring that:
    /// 1. the registry contains all necessary public keys
    /// 2. the public keys coming from the registry match the ones stored in the local public key store
    /// 3. the secret key store contains all corresponding secret keys.
    ///
    /// Returns the status of the public keys as follows:
    /// * [`AllKeysRegistered`]:
    /// Registry contains all required public keys and
    /// secret key store contains all corresponding secret keys.
    /// * [`IDkgDealingEncPubkeyNeedsRegistration`]:
    /// All keys are properly set up like in [`AllKeysRegistered`] except for the
    /// I-DKG dealing encryption key which is available locally in the public key store
    /// but not yet in the registry and therefore needs to be registered.
    /// * [`RotateIDkgDealingEncryptionKeys`]:
    /// All keys are properly set up like in [`AllKeysRegistered`]
    /// but the I-DKG dealing encryption key coming from the registry is too old
    /// and a new I-DKG dealing key pair must be generated.
    ///
    /// [`AllKeysRegistered`]: PublicKeyRegistrationStatus::AllKeysRegistered
    /// [`IDkgDealingEncPubkeyNeedsRegistration`]: PublicKeyRegistrationStatus::IDkgDealingEncPubkeyNeedsRegistration
    /// [`RotateIDkgDealingEncryptionKeys`]: PublicKeyRegistrationStatus::RotateIDkgDealingEncryptionKeys
    ///
    /// # Errors
    /// See [`ic_types::crypto::CryptoError`].
    fn check_keys_with_registry(
        &self,
        registry_version: RegistryVersion,
    ) -> CryptoResult<PublicKeyRegistrationStatus>;

    /// Returns the node's public keys currently stored in the public key store.
    ///
    /// Calling this method multiple times may lead to different results
    /// depending on the state of the public key store.
    ///
    /// # Errors
    /// * [`CurrentNodePublicKeysError::TransientInternalError`] in case of a transient internal error.
    fn current_node_public_keys(&self)
        -> Result<CurrentNodePublicKeys, CurrentNodePublicKeysError>;

    /// Rotates the I-DKG dealing encryption keys. This function shall only be called if a prior
    /// call to `check_keys_with_registry()` has indicated that the I-DKG dealing encryption keys
    /// shall be rotated. Returns a `PublicKeyProto` containing the new I-DKG dealing encryption
    /// key to be registered, or an error if the key rotation failed.
    ///
    /// # Errors
    /// * `IDkgDealingEncryptionKeyRotationError::LatestLocalRotationTooRecent` if the node local
    ///   I-DKG dealing encryption keys are too recent, and the keys cannot be rotated. The caller
    ///   needs to wait longer before the keys can be rotated. To determine whether or not the
    ///   I-DKG dealing encryption keys can be rotated, inspect the return value of
    ///   `check_keys_with_registry`.
    fn rotate_idkg_dealing_encryption_keys(
        &self,
        registry_version: RegistryVersion,
    ) -> Result<PublicKeyProto, IDkgDealingEncryptionKeyRotationError>;

    /// Returns the number of iDKG dealing encryption public keys stored locally.
    ///
    /// # Errors
    /// * if a transient error (e.g., RPC timeout) occurs when accessing the public key store
    fn idkg_dealing_encryption_pubkeys_count(
        &self,
    ) -> Result<usize, IdkgDealingEncPubKeysCountError>;
}

#[derive(Clone, Debug)]
pub enum PublicKeyRegistrationStatus {
    AllKeysRegistered,
    IDkgDealingEncPubkeyNeedsRegistration(PublicKeyProto),
    RotateIDkgDealingEncryptionKeys,
}

#[derive(Clone, Debug)]
pub enum IDkgDealingEncryptionKeyRotationError {
    LatestLocalRotationTooRecent,
    KeyGenerationError(String),
    RegistryError(RegistryClientError),
    KeyRotationNotEnabled,
    TransientInternalError(String),
}

impl From<RegistryClientError> for IDkgDealingEncryptionKeyRotationError {
    fn from(registry_client_error: RegistryClientError) -> Self {
        IDkgDealingEncryptionKeyRotationError::RegistryError(registry_client_error)
    }
}
