#![cfg_attr(target_family = "wasm", no_std)]

use soroban_sdk::{contract, contractimpl, contracttype, contracterror, token, Address, Env};

#[contracterror]
#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u32)]
pub enum Error {
    AlreadySubscribed = 1,
    NotSubscribed = 2,
    PaymentNotDue = 3,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct Subscription {
    pub subscriber: Address,
    pub merchant: Address,
    pub token: Address,
    pub amount: i128,
    pub interval: u64,
    pub next_payment: u64,
}

#[contracttype]
pub enum DataKey {
    Sub(Address),
}

#[contract]
pub struct SubscriptionContract;

#[contractimpl]
impl SubscriptionContract {
    /// Create a new subscription. The first payment is due immediately (next_payment = now).
    pub fn subscribe(
        env: Env,
        subscriber: Address,
        merchant: Address,
        token: Address,
        amount: i128,
        interval: u64,
    ) -> Result<(), Error> {
        subscriber.require_auth();

        let key = DataKey::Sub(subscriber.clone());
        if env.storage().persistent().has(&key) {
            return Err(Error::AlreadySubscribed);
        }

        let now = env.ledger().timestamp();
        let sub = Subscription {
            subscriber,
            merchant,
            token,
            amount,
            interval,
            next_payment: now,
        };
        env.storage().persistent().set(&key, &sub);
        Ok(())
    }

    /// Execute a payment. Fails with PaymentNotDue if called before next_payment.
    pub fn execute_payment(env: Env, subscriber: Address) -> Result<(), Error> {
        let key = DataKey::Sub(subscriber.clone());
        let mut sub: Subscription = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(Error::NotSubscribed)?;

        let now = env.ledger().timestamp();
        if now < sub.next_payment {
            return Err(Error::PaymentNotDue);
        }

        let token_client = token::Client::new(&env, &sub.token);
        token_client.transfer(&sub.subscriber, &sub.merchant, &sub.amount);

        sub.next_payment = now + sub.interval;
        env.storage().persistent().set(&key, &sub);
        Ok(())
    }

    /// Cancel an active subscription.
    pub fn cancel(env: Env, subscriber: Address) -> Result<(), Error> {
        subscriber.require_auth();

        let key = DataKey::Sub(subscriber);
        if !env.storage().persistent().has(&key) {
            return Err(Error::NotSubscribed);
        }

        env.storage().persistent().remove(&key);
        Ok(())
    }

    /// Read the stored subscription record.
    pub fn get_subscription(env: Env, subscriber: Address) -> Option<Subscription> {
        env.storage()
            .persistent()
            .get(&DataKey::Sub(subscriber))
    }
}

mod test;
