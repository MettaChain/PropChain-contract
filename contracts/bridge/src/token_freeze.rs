use ink::storage::Mapping;

pub struct TokenFreezeManager {
    pub frozen_tokens: Mapping<u128, bool>,
}

impl Default for TokenFreezeManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenFreezeManager {
    pub fn new() -> Self {
        Self {
            frozen_tokens: Mapping::default(),
        }
    }

    pub fn freeze_token(&mut self, token_id: u128) {
        self.frozen_tokens.insert(token_id, &true);
    }

    pub fn is_token_frozen(&self, token_id: u128) -> bool {
        self.frozen_tokens.get(token_id).unwrap_or(false)
    }
}
