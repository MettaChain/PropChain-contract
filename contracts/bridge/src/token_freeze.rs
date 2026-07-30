use ink::storage::Mapping;

pub struct TokenFreezeManager {
    pub frozen_tokens: Mapping<u64, bool>,
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

    pub fn freeze_token(&mut self, token_id: u64) {
        self.frozen_tokens.insert(token_id, &true);
    }

    pub fn unfreeze_token(&mut self, token_id: u64) {
        self.frozen_tokens.insert(token_id, &false);
    }

    pub fn is_token_frozen(&self, token_id: u64) -> bool {
        self.frozen_tokens.get(token_id).unwrap_or(false)
    }
}
