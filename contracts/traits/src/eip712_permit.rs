use crate::AccountId;

pub struct Eip712Permit {
    pub chain_id: u64,
    pub nonce: u64,
    pub owner: AccountId,
    pub spender: AccountId,
    pub value: u128,
    pub deadline: u64,
}

impl Eip712Permit {
    pub fn verify_permit(&self, current_chain_id: u64, expected_nonce: u64) -> bool {
        self.chain_id == current_chain_id && self.nonce == expected_nonce
    }
}
