// SPDX-License-Identifier: MIT

/// EIP-712 permit data for gasless approvals.
///
/// Allows a user to authorise a spender by signing a typed EIP-712 message
/// off-chain, then submitting it on-chain via a relayer — the user never
/// needs to pay gas for the approval transaction.
pub struct Eip712Permit {
    /// Chain ID to prevent replay attacks across chains.
    pub chain_id: u64,
    /// Nonce scoped to the owner account.
    pub nonce: u64,
    /// Address of the token owner who signed the permit.
    pub owner: crate::AccountId,
    /// Address of the spender being authorised.
    pub spender: crate::AccountId,
    /// Maximum amount the spender may transfer.
    pub value: u128,
    /// Deadline UNIX timestamp after which the permit is invalid.
    pub deadline: u64,
}

impl Eip712Permit {
    /// Verify that the permit is valid for the given chain and nonce.
    pub fn verify_permit(&self, current_chain_id: u64, expected_nonce: u64) -> bool {
        self.chain_id == current_chain_id && self.nonce == expected_nonce
    }
}
