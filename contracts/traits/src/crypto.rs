use ink::primitives::{AccountId, Hash};
use ink::prelude::vec::Vec;

// ── Error Types ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub enum CryptoError {
    /// ECDSA signature recovery failed
    InvalidSignature,
    /// Recovered public key does not match the registered key
    InvalidPublicKey,
    /// Hash computation failed
    HashError,
    /// Key rotation is still in cooldown period
    KeyRotationCooldown,
    /// Key rotation request has expired
    KeyRotationExpired,
    /// No pending key rotation for this account
    NoPendingRotation,
    /// Caller is not authorized for this key rotation action
    RotationUnauthorized,
    /// Randomness round is not in the expected phase
    InvalidRandomnessPhase,
    /// Commit does not match revealed secret
    CommitMismatch,
    /// Not enough participants revealed their secrets
    InsufficientReveals,
}

impl core::fmt::Display for CryptoError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            CryptoError::InvalidSignature => write!(f, "ECDSA signature recovery failed"),
            CryptoError::InvalidPublicKey => {
                write!(f, "Recovered public key does not match registered key")
            }
            CryptoError::HashError => write!(f, "Hash computation failed"),
            CryptoError::KeyRotationCooldown => {
                write!(f, "Key rotation is still in cooldown period")
            }
            CryptoError::KeyRotationExpired => write!(f, "Key rotation request has expired"),
            CryptoError::NoPendingRotation => {
                write!(f, "No pending key rotation for this account")
            }
            CryptoError::RotationUnauthorized => {
                write!(f, "Caller is not authorized for this key rotation action")
            }
            CryptoError::InvalidRandomnessPhase => {
                write!(f, "Randomness round is not in the expected phase")
            }
            CryptoError::CommitMismatch => {
                write!(f, "Commit does not match the revealed secret")
            }
            CryptoError::InsufficientReveals => {
                write!(f, "Not enough participants revealed their secrets")
            }
        }
    }
}

// ── Audit Types ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub enum CryptoAuditAction {
    HashComputed,
    SignatureVerified,
    SignatureRejected,
    KeyRotationRequested,
    KeyRotationCompleted,
    KeyRotationCancelled,
    RandomnessCommitted,
    RandomnessRevealed,
    RandomnessFinalized,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub enum HashAlgorithm {
    Blake2b256,
    Keccak256,
}

#[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct CryptoAuditEvent {
    pub action: CryptoAuditAction,
    pub actor: AccountId,
    pub target_hash: Option<Hash>,
    pub algorithm: Option<HashAlgorithm>,
    pub success: bool,
    pub block_number: u32,
    pub timestamp: u64,
}

// ── Signature Types ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct SignedApproval {
    pub signature: [u8; 65],
    pub message_hash: [u8; 32],
}

// ── Key Rotation Types ──────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct KeyRotationRequest {
    pub old_account: AccountId,
    pub new_account: AccountId,
    pub requested_at: u32,
    pub effective_at: u32,
    pub confirmed: bool,
}

// ── Hash Functions ──────────────────────────────────────────────────────────

/// Compute a Blake2b-256 hash of raw bytes.
///
/// Blake2b-256 is Substrate's native hash function and the cheapest to
/// compute in ink! WASM (no additional host function overhead).
pub fn hash_blake2b256(data: &[u8]) -> Hash {
    let mut output = <ink::env::hash::Blake2x256 as ink::env::hash::HashOutput>::Type::default();
    ink::env::hash_bytes::<ink::env::hash::Blake2x256>(data, &mut output);
    Hash::from(output)
}

/// Compute a Keccak-256 hash of raw bytes.
///
/// Useful for Ethereum-compatible hashing (e.g., bridge operations).
pub fn hash_keccak256(data: &[u8]) -> Hash {
    let mut output = <ink::env::hash::Keccak256 as ink::env::hash::HashOutput>::Type::default();
    ink::env::hash_bytes::<ink::env::hash::Keccak256>(data, &mut output);
    Hash::from(output)
}

/// SCALE-encode a value and compute its Blake2b-256 hash.
///
/// This is the recommended way to hash structured data in PropChain contracts.
/// It replaces the previous pattern of truncating SCALE-encoded bytes to 32 bytes,
/// which was not a cryptographic hash and could produce collisions.
pub fn hash_encoded<T: scale::Encode>(value: &T) -> Hash {
    let encoded = value.encode();
    hash_blake2b256(&encoded)
}

// ── Signature Verification ──────────────────────────────────────────────────

/// Recover the compressed ECDSA public key from a recoverable signature.
///
/// Returns the 33-byte compressed public key on success.
/// The caller is responsible for checking that the recovered key matches
/// an expected/registered public key.
pub fn verify_ecdsa_signature(
    signature: &[u8; 65],
    message_hash: &[u8; 32],
) -> Result<[u8; 33], CryptoError> {
    let mut output = [0u8; 33];
    ink::env::ecdsa_recover(signature, message_hash, &mut output)
        .map_err(|_| CryptoError::InvalidSignature)?;
    Ok(output)
}

/// Verify that an ECDSA signature was produced by the owner of a registered public key.
///
/// Computes the expected message hash from the provided data, recovers the
/// public key from the signature, and checks it against the expected key.
pub fn verify_signed_approval(
    approval: &SignedApproval,
    expected_public_key: &[u8; 33],
) -> Result<(), CryptoError> {
    let recovered = verify_ecdsa_signature(&approval.signature, &approval.message_hash)?;
    if recovered != *expected_public_key {
        return Err(CryptoError::InvalidPublicKey);
    }
    Ok(())
}

// ── Commitment-Reveal Helpers ───────────────────────────────────────────────

/// Compute a commitment hash for a secret value and sender address.
///
/// The commitment is `Blake2b256(secret || sender)` to prevent front-running.
pub fn compute_commitment(secret: &[u8; 32], sender: &AccountId) -> Hash {
    let mut data = Vec::with_capacity(64);
    data.extend_from_slice(secret);
    data.extend_from_slice(sender.as_ref());
    hash_blake2b256(&data)
}

/// Verify that a revealed secret matches a previously submitted commitment.
pub fn verify_commitment(secret: &[u8; 32], sender: &AccountId, commitment: &Hash) -> bool {
    compute_commitment(secret, sender) == *commitment
}

/// Finalize randomness from multiple revealed secrets by XOR-ing and hashing.
pub fn finalize_randomness(secrets: &[[u8; 32]]) -> Hash {
    let mut xored = [0u8; 32];
    for secret in secrets {
        for (i, byte) in secret.iter().enumerate() {
            xored[i] ^= byte;
        }
    }
    hash_blake2b256(&xored)
}
