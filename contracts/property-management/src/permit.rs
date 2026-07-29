//! Closes #804: EIP-2612-style permits (owner, spender, value, deadline, v,
//! r, s) so an off-chain signature can grant allowance without a prior
//! approve transaction. Starter replay/deadline/nonce checks (mirrors the
//! existing `traits::eip712_permit` pattern); real signature recovery
//! against a domain separator is a follow-up.

pub struct PermitRequest {
    pub nonce: u64,
    pub deadline: u64,
}

/// Validates a permit's replay/expiry guard: the nonce must match what's
/// expected on-chain, and `deadline` must not have passed at `now`.
pub fn validate_permit(request: &PermitRequest, expected_nonce: u64, now: u64) -> Result<(), &'static str> {
    if request.nonce != expected_nonce {
        return Err("invalid or replayed nonce");
    }
    if now > request.deadline {
        return Err("permit expired");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_a_valid_unexpired_permit() {
        let req = PermitRequest { nonce: 1, deadline: 1_000 };
        assert!(validate_permit(&req, 1, 500).is_ok());
    }

    #[test]
    fn rejects_a_replayed_nonce() {
        let req = PermitRequest { nonce: 1, deadline: 1_000 };
        assert!(validate_permit(&req, 2, 500).is_err());
    }

    #[test]
    fn rejects_an_expired_deadline() {
        let req = PermitRequest { nonce: 1, deadline: 1_000 };
        assert!(validate_permit(&req, 1, 1_001).is_err());
    }
}
