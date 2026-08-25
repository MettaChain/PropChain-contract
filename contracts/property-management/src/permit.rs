// Closes #804 and #995: EIP-2612-style permits (owner, spender, value,
// deadline, v, r, s) so an off-chain signature can grant allowance without a
// prior approve transaction.
//
// Reconciled with the corrected `traits::eip712_permit` API (#995): a permit
// is only accepted when its ECDSA signature recovers to the owner's
// registered public key over the domain-separated payload hash, in addition
// to the replay/deadline/nonce guards.

use ink::primitives::Hash;
use propchain_traits::{AccountId, Eip712Permit};

/// A complete permit authorization request: the signed payload fields plus
/// the owner's registered public key and the raw 65-byte signature.
pub struct PermitRequest {
    pub chain_id: u64,
    pub nonce: u64,
    pub owner: AccountId,
    pub spender: AccountId,
    pub value: u128,
    pub deadline: u64,
    pub owner_public_key: [u8; 33],
    pub signature: [u8; 65],
}

impl PermitRequest {
    /// The domain-separated digest that must have been signed off-chain.
    pub fn message_hash(&self) -> Hash {
        Eip712Permit {
            chain_id: self.chain_id,
            nonce: self.nonce,
            owner: self.owner.clone(),
            spender: self.spender.clone(),
            value: self.value,
            deadline: self.deadline,
        }
        .message_hash()
    }

    /// Validates a permit end to end:
    ///
    /// 1. the nonce matches what is expected on-chain (replay guard),
    /// 2. `deadline` has not passed at `now`,
    /// 3. `signature` recovers to `owner_public_key` over the payload hash
    ///    binding chain id, nonce, owner, spender, value and deadline.
    pub fn validate_permit(
        &self,
        expected_nonce: u64,
        current_chain_id: u64,
        now: u64,
    ) -> Result<(), &'static str> {
        if self.nonce != expected_nonce {
            return Err("invalid or replayed nonce");
        }
        if now > self.deadline {
            return Err("permit expired");
        }
        let permit = Eip712Permit {
            chain_id: self.chain_id,
            nonce: self.nonce,
            owner: self.owner.clone(),
            spender: self.spender.clone(),
            value: self.value,
            deadline: self.deadline,
        };
        if !permit.verify_permit(
            current_chain_id,
            expected_nonce,
            now,
            &self.owner_public_key,
            &self.signature,
        ) {
            return Err("signature verification failed");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use secp256k1::{Message, Secp256k1, SecretKey};

    fn sign(index: u8, hash: &[u8; 32]) -> ([u8; 33], [u8; 65]) {
        let engine = Secp256k1::signing_only();
        let mut secret = [0u8; 32];
        secret[31] = index + 1;
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

    fn signed_request(signer_index: u8) -> PermitRequest {
        let probe = PermitRequest {
            chain_id: 1,
            nonce: 1,
            owner: AccountId::from([0x11; 32]),
            spender: AccountId::from([0x22; 32]),
            value: 500,
            deadline: 1_000,
            owner_public_key: [0u8; 33],
            signature: [0u8; 65],
        };
        let hash = <[u8; 32]>::from(probe.message_hash());
        let (public_key, signature) = sign(signer_index, &hash);
        PermitRequest {
            owner_public_key: public_key,
            signature,
            ..probe
        }
    }

    #[test]
    fn accepts_a_valid_unexpired_permit() {
        let req = signed_request(1);
        assert!(req.validate_permit(1, 1, 500).is_ok());
        // Boundary: still valid exactly at the deadline.
        assert!(req.validate_permit(1, 1, 1_000).is_ok());
    }

    #[test]
    fn rejects_a_replayed_nonce() {
        let req = signed_request(1);
        assert_eq!(
            req.validate_permit(2, 1, 500),
            Err("invalid or replayed nonce")
        );
    }

    #[test]
    fn rejects_an_expired_deadline() {
        let req = signed_request(1);
        assert_eq!(req.validate_permit(1, 1, 1_001), Err("permit expired"));
    }

    #[test]
    fn rejects_a_forged_signature() {
        // The owner's key stays registered, but the payload was signed by
        // someone else.
        let req = signed_request(1);
        let hash = <[u8; 32]>::from(req.message_hash());
        let (_, forged_sig) = sign(2, &hash);
        let forged = PermitRequest {
            signature: forged_sig,
            ..req
        };
        assert_eq!(
            forged.validate_permit(1, 1, 500),
            Err("signature verification failed")
        );
    }

    #[test]
    fn rejects_a_signature_over_a_tampered_payload() {
        let mut req = signed_request(1);
        // Same owner key/signature but a different amount was signed.
        req.value = 999_999;
        assert_eq!(
            req.validate_permit(1, 1, 500),
            Err("signature verification failed")
        );
    }
}
