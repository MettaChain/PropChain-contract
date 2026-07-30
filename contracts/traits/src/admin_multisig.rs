use ink::prelude::vec::Vec;

use crate::AccountId;

pub trait MultiSigAdminRotationTrait {
    fn confirm_key_rotation(&mut self, approvals: Vec<AccountId>) -> Result<(), &'static str>;
}
