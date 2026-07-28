use ink::storage::Mapping;
use propchain_traits::AccountId;

pub struct ConstantTimeSanctionsMap {
    pub sanctioned_accounts: Mapping<AccountId, bool>,
}

impl Default for ConstantTimeSanctionsMap {
    fn default() -> Self {
        Self::new()
    }
}

impl ConstantTimeSanctionsMap {
    pub fn new() -> Self {
        Self {
            sanctioned_accounts: Mapping::default(),
        }
    }

    pub fn is_sanctioned(&self, account: AccountId) -> bool {
        self.sanctioned_accounts.get(account).unwrap_or(false)
    }

    pub fn set_sanctioned(&mut self, account: AccountId, sanctioned: bool) {
        self.sanctioned_accounts.insert(account, &sanctioned);
    }
}
