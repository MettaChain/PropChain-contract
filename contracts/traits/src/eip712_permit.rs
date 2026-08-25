use ink::primitives::Hash;

use crate::{crypto, AccountId};

/// EIP-2612-style permit payload (Issue #995).
///
/// The struct is only a *payload description*: on its own it proves nothing.
/// Authorization requires an ECDSA signature over [`Eip712Permit::message_hash`]
/// that recovers to the owner's registered compressed public key, plus chain,
/// nonce and deadline checks. See `verify_permit`.
#[derive(Clone, Debug)]
pub struct Eip712Permit {
    pub chain_id: u64,
    pub nonce: u64,
    pub owner: AccountId,
    pub spender: AccountId,
    pub value: u128,
    pub deadline: u64,
}

impl Eip712Permit {
    /// Domain-separated digest binding *every* field of the permit
    /// (chain id, nonce, owner, spender, value, deadline). A signature over
    /// this hash cannot be replayed against a different spender, value or
    /// chain because each is part of the signed message.
    pub fn message_hash(&self) -> Hash {
        crypto::hash_encoded(&(
            b"PropChain:Eip712Permit:v1",
            self.chain_id,
            self.nonce,
            self.owner,
            self.spender,
            self.value,
            self.deadline,
        ))
    }

    /// Fully verify a permit:
    ///
    /// 1. the permit targets the current chain (`current_chain_id`),
    /// 2. its nonce matches the on-chain expectation (replay guard),
    /// 3. it has not expired (`now <= deadline`),
    /// 4. `signature` recovers to `owner_public_key` over the domain-separated
    ///    [`Eip712Permit::message_hash`].
    ///
    /// The workspace account model stores explicit `[u8; 33]` public keys per
    /// account (see governance `register_public_key`), so the caller resolves
    /// `owner` to its registered key; recovery guarantees the signature was
    /// produced by that key's secret. The spender/value are authenticated
    /// transitively: they are part of the signed hash, so a signature valid
    /// for one permit cannot authorize a different one.
    pub fn verify_permit(
        &self,
        current_chain_id: u64,
        expected_nonce: u64,
        now: u64,
        owner_public_key: &[u8; 33],
        signature: &[u8; 65],
    ) -> bool {
        if self.chain_id != current_chain_id || self.nonce != expected_nonce {
            return false;
        }
        if now > self.deadline {
            return false;
        }
        let message_hash = <[u8; 32]>::from(self.message_hash());
        match crypto::verify_ecdsa_signature(signature, &message_hash) {
            Ok(recovered) => recovered == *owner_public_key,
            Err(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use secp256k1::{Message, Secp256k1, SecretKey};

    use super::*;

    /// Sign `hash` with the secret for `index` and return
    /// (public_key, signature) in the shapes the contract expects.
    fn sign(index: u8, hash: &[u8; 32]) -> ([u8; 33], [u8; 65]) {
        let engine = Secp256k1::signing_only();
        let mut secret = [0u8; 32];
        secret[31] = index + 1; // deterministic distinct keys
        let secret = SecretKey::from_slice(&secret).expect("valid secret");
        let msg = Message::from_digest_slice(hash).expect("valid message");
        let sig = engine.sign_ecdsa_recoverable(&msg, &secret);
        let (recovery_id, data) = sig.serialize_compact();
        let mut signature = [0u8; 65];
        signature[..64].copy_from_slice(&data);
        signature[64] = recovery_id.to_i32() as u8;
        let public = secp256k1::PublicKey::from_secret_key(&engine, &secret);
        (public.serialize(), signature)
    }

    fn sample_permit(owner: AccountId, spender: AccountId) -> Eip712Permit {
        Eip712Permit {
            chain_id: 1,
            nonce: 7,
            owner,
            spender,
            value: 1_000,
            deadline: 5_000,
        }
    }

    #[test]
    fn valid_signature_from_owner_passes() {
        let owner = AccountId::from([0x11; 32]);
        let spender = AccountId::from([0x22; 32]);
        let permit = sample_permit(owner, spender);
        let hash = <[u8; 32]>::from(permit.message_hash());
        let (public_key, signature) = sign(1, &hash);

        assert!(permit.verify_permit(1, 7, 4_999, &public_key, &signature));
        // Boundary: at exactly the deadline the permit is still usable.
        assert!(permit.verify_permit(1, 7, 5_000, &public_key, &signature));
    }

    #[test]
    fn forged_signature_from_wrong_signer_fails() {
        let owner = AccountId::from([0x11; 32]);
        let spender = AccountId::from([0x22; 32]);
        let permit = sample_permit(owner, spender);
        let hash = <[u8; 32]>::from(permit.message_hash());

        // Signed by someone who is not the owner.
        let (_attacker_key, attacker_sig) = sign(2, &hash);
        let (owner_key, _owner_sig) = sign(1, &hash);
        assert!(!permit.verify_permit(1, 7, 4_999, &owner_key, &attacker_sig));
    }

    #[test]
    fn signature_over_tampered_payload_fails() {
        let owner = AccountId::from([0x11; 32]);
        let spender = AccountId::from([0x22; 32]);
        let permit = sample_permit(owner, spender);
        let hash = <[u8; 32]>::from(permit.message_hash());
        let (public_key, signature) = sign(1, &hash);

        // Same key/signature but the permit now names a different spender or
        // value: the payload hash no longer matches the signed one.
        let mut tampered = sample_permit(owner, spender);
        tampered.value = 999_999;
        assert!(!tampered.verify_permit(1, 7, 4_999, &public_key, &signature));

        let mut other_spender = sample_permit(owner, spender);
        other_spender.spender = AccountId::from([0x33; 32]);
        assert!(!other_spender.verify_permit(1, 7, 4_999, &public_key, &signature));
    }

    #[test]
    fn wrong_chain_nonce_or_deadline_fails() {
        let owner = AccountId::from([0x11; 32]);
        let spender = AccountId::from([0x22; 32]);
        let permit = sample_permit(owner, spender);
        let hash = <[u8; 32]>::from(permit.message_hash());
        let (public_key, signature) = sign(1, &hash);

        // Wrong chain id / replayed nonce.
        assert!(!permit.verify_permit(2, 7, 4_999, &public_key, &signature));
        assert!(!permit.verify_permit(1, 8, 4_999, &public_key, &signature));

        // Expired: strictly past the deadline.
        assert!(!permit.verify_permit(1, 7, 5_001, &public_key, &signature));
    }

    #[test]
    fn message_hash_binds_all_fields() {
        let base = sample_permit(AccountId::from([0x11; 32]), AccountId::from([0x22; 32]));
        let base_hash = base.message_hash();

        let mut changed = base.clone();
        changed.value += 1;
        assert_ne!(base_hash, changed.message_hash());

        let mut changed = base.clone();
        changed.deadline += 1;
        assert_ne!(base_hash, changed.message_hash());

        let mut changed = base.clone();
        changed.spender = AccountId::from([0x44; 32]);
        assert_ne!(base_hash, changed.message_hash());
    }
}
