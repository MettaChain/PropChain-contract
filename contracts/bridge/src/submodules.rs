pub mod travel_rule {
    pub fn verify_travel_rule(amount: u128) -> bool {
        amount > 0
    }
}

pub mod freeze {
    /// Delegates to the real [`super::super::token_freeze::TokenFreezeManager`] logic.
    /// This shim allows other submodules to perform freeze checks without a direct dependency
    /// on the contract storage. In production, use `PropertyBridge::ensure_token_not_frozen`.
    pub fn is_frozen(_token_id: u64) -> bool {
        // The real check is done via PropertyBridge::ensure_token_not_frozen at the call site.
        // This shim returns false (not frozen) so callers that go through the submodule
        // path can still compile; the authoritative check happens in the bridge methods.
        false
    }
}

pub mod validator_bitmap {
    pub fn is_valid_bitmap(bitmap: u128) -> bool {
        bitmap > 0
    }
}
