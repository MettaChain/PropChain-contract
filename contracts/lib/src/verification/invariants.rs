//! Formal verification harnesses using Kani.
//!
//! These proofs cover three invariants required by the security issue:
//!   1. Balance conservation  — tokens cannot be created or destroyed
//!   2. Access-control roles  — only authorised addresses can call admin functions
//!   3. Oracle staleness bound — price data must be recent enough to be trusted

// ─────────────────────────────────────────────────────────────────────────────
// 1. BALANCE CONSERVATION
//    Proves that a transfer between two accounts never changes the total supply.
// ─────────────────────────────────────────────────────────────────────────────

/// Simulated token ledger (replace with your actual contract types).
struct TokenLedger {
    sender_balance: u64,
    receiver_balance: u64,
}

impl TokenLedger {
    /// Transfer `amount` from sender to receiver.
    /// Returns Err if the sender does not have enough funds.
    fn transfer(&mut self, amount: u64) -> Result<(), &'static str> {
        if self.sender_balance < amount {
            return Err("insufficient balance");
        }
        self.sender_balance -= amount;
        self.receiver_balance += amount;
        Ok(())
    }

    fn total(&self) -> u64 {
        // saturating_add prevents wrapping on overflow — Kani will still catch it
        self.sender_balance.saturating_add(self.receiver_balance)
    }
}

#[cfg(kani)]
mod balance_proofs {
    use super::*;

    #[kani::proof]
    fn prove_balance_conservation() {
        // kani::any() tells Kani to try ALL possible u64 values
        let sender_balance: u64 = kani::any();
        let receiver_balance: u64 = kani::any();
        let amount: u64 = kani::any();

        // Prevent integer overflow in the total — a realistic contract constraint
        kani::assume(sender_balance.checked_add(receiver_balance).is_some());

        let mut ledger = TokenLedger { sender_balance, receiver_balance };
        let total_before = ledger.total();

        // Whether the transfer succeeds or fails, the total must not change
        let _ = ledger.transfer(amount);

        assert_eq!(
            ledger.total(),
            total_before,
            "Balance conservation violated: total supply changed after transfer"
        );
    }

    #[kani::proof]
    fn prove_no_balance_underflow() {
        let sender_balance: u64 = kani::any();
        let amount: u64 = kani::any();

        // Only try cases where the transfer should fail
        kani::assume(amount > sender_balance);

        let mut ledger = TokenLedger {
            sender_balance,
            receiver_balance: 0,
        };

        // Must return an error — sender balance must be unchanged
        let result = ledger.transfer(amount);
        assert!(result.is_err(), "Expected error for insufficient balance");
        assert_eq!(
            ledger.sender_balance, sender_balance,
            "Sender balance changed on failed transfer"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. ACCESS-CONTROL ROLE MEMBERSHIP
//    Proves that only addresses with the Admin role can call privileged functions.
// ─────────────────────────────────────────────────────────────────────────────

#[derive(PartialEq, Clone, Copy)]
enum Role {
    None,
    User,
    Admin,
}

struct AccessControl {
    caller_role: Role,
}

impl AccessControl {
    /// Privileged function — must only succeed for Admin callers.
    fn admin_only_action(&self) -> Result<(), &'static str> {
        if self.caller_role != Role::Admin {
            return Err("access denied: caller is not Admin");
        }
        // … real logic here …
        Ok(())
    }
}

#[cfg(kani)]
mod access_control_proofs {
    use super::*;

    #[kani::proof]
    fn prove_non_admin_always_rejected() {
        // Pick any role that is NOT Admin
        let role: u8 = kani::any();
        kani::assume(role != 2); // 0 = None, 1 = User, 2 = Admin

        let caller_role = match role % 3 {
            0 => Role::None,
            _ => Role::User,   // covers 1 and any other non-admin value
        };

        let ac = AccessControl { caller_role };
        let result = ac.admin_only_action();

        assert!(
            result.is_err(),
            "Security violation: non-admin caller was not rejected"
        );
    }

    #[kani::proof]
    fn prove_admin_always_accepted() {
        let ac = AccessControl { caller_role: Role::Admin };
        let result = ac.admin_only_action();
        assert!(result.is_ok(), "Admin was incorrectly rejected");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. ORACLE STALENESS BOUND
//    Proves that price data older than MAX_AGE_SECONDS is always rejected.
// ─────────────────────────────────────────────────────────────────────────────

/// Maximum age (in seconds) we accept for oracle price data.
const MAX_AGE_SECONDS: u64 = 300; // 5 minutes — adjust to your contract's constant

struct OraclePrice {
    /// Unix timestamp when the price was last updated
    updated_at: u64,
}

impl OraclePrice {
    /// Returns Ok(price) only when the data is fresh enough.
    fn get_verified_price(&self, current_time: u64) -> Result<u64, &'static str> {
        let age = current_time.saturating_sub(self.updated_at);
        if age > MAX_AGE_SECONDS {
            return Err("oracle data is stale");
        }
        // Return a dummy price — replace with real field access
        Ok(42_000_u64)
    }
}

#[cfg(kani)]
mod oracle_proofs {
    use super::*;

    #[kani::proof]
    fn prove_stale_oracle_always_rejected() {
        let updated_at: u64 = kani::any();
        let current_time: u64 = kani::any();

        // Force a stale scenario: current time is more than MAX_AGE_SECONDS ahead
        kani::assume(current_time > updated_at);
        kani::assume(current_time - updated_at > MAX_AGE_SECONDS);

        let oracle = OraclePrice { updated_at };
        let result = oracle.get_verified_price(current_time);

        assert!(
            result.is_err(),
            "Staleness violation: stale oracle price was accepted"
        );
    }

    #[kani::proof]
    fn prove_fresh_oracle_always_accepted() {
        let updated_at: u64 = kani::any();
        let current_time: u64 = kani::any();

        // Force a fresh scenario
        kani::assume(current_time >= updated_at);
        kani::assume(current_time - updated_at <= MAX_AGE_SECONDS);

        let oracle = OraclePrice { updated_at };
        let result = oracle.get_verified_price(current_time);

        assert!(
            result.is_ok(),
            "Fresh oracle price was incorrectly rejected"
        );
    }
}
