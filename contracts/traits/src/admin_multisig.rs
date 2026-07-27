use crate::AccountId;
use ink::prelude::vec::Vec;

pub trait MultiSigAdminRotationTrait {
    fn confirm_key_rotation(&mut self, approvals: Vec<AccountId>) -> Result<(), &'static str>;
}
