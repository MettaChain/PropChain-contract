#![cfg_attr(not(feature = "std"), no_std)]

use ink::prelude::string::String;
use ink::storage::Mapping;

#[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
pub struct RateLimitBucket {
    pub tokens: u32,
    pub last_refill: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
pub struct RateLimitConfig {
    pub max_tokens: u32,
    pub refill_rate: u32,
    pub global_max_tokens: u32,
}

#[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum RateLimitError {
    RateLimitExceeded,
}

pub struct RateLimiter {
    pub user_rate_limits: Mapping<[u8; 32], RateLimitBucket>,
    pub global_rate_limit: RateLimitBucket,
    pub config: RateLimitConfig,
    pub bypass_enabled: bool,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            user_rate_limits: Mapping::default(),
            global_rate_limit: RateLimitBucket {
                tokens: 1000,
                last_refill: 0,
            },
            config: RateLimitConfig {
                max_tokens: 100,
                refill_rate: 5,
                global_max_tokens: 1000,
            },
            bypass_enabled: false,
        }
    }

    pub fn check_rate_limit(
        &mut self,
        user: [u8; 32],
        now: u64,
        operation: String,
    ) -> Result<(), RateLimitError> {
        if self.bypass_enabled {
            return Ok(());
        }

        // Global bucket
        self.refill_bucket(&mut self.global_rate_limit, now, self.config.global_max_tokens);

        if self.global_rate_limit.tokens == 0 {
            return Err(RateLimitError::RateLimitExceeded);
        }

        self.global_rate_limit.tokens -= 1;

        // User bucket
        let mut bucket = self.user_rate_limits.get(&user).unwrap_or(RateLimitBucket {
            tokens: self.config.max_tokens,
            last_refill: now,
        });

        self.refill_bucket(&mut bucket, now, self.config.max_tokens);

        if bucket.tokens == 0 {
            return Err(RateLimitError::RateLimitExceeded);
        }

        bucket.tokens -= 1;
        self.user_rate_limits.insert(&user, &bucket);

        Ok(())
    }

    fn refill_bucket(&self, bucket: &mut RateLimitBucket, now: u64, max_tokens: u32) {
        let elapsed = now.saturating_sub(bucket.last_refill);
        let refill = (elapsed as u32) * self.config.refill_rate;

        if refill > 0 {
            bucket.tokens = core::cmp::min(bucket.tokens + refill, max_tokens);
            bucket.last_refill = now;
        }
    }

    pub fn set_bypass(&mut self, enabled: bool) {
        self.bypass_enabled = enabled;
    }

    pub fn update_config(&mut self, config: RateLimitConfig) {
        self.config = config;
    }

    pub fn get_status(&self, user: [u8; 32]) -> (u32, u32) {
        let user_tokens = self
            .user_rate_limits
            .get(&user)
            .map(|b| b.tokens)
            .unwrap_or(self.config.max_tokens);

        let global_tokens = self.global_rate_limit.tokens;

        (user_tokens, global_tokens)
    }
}
