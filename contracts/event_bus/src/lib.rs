#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod event_bus_contract {
    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;
    use propchain_contracts::{non_reentrant, ReentrancyError, ReentrancyGuard};
    use propchain_traits::event_bus::{
        EventBus, EventBusError, EventPayload, EventSubscriberRef, Topic,
    };

    const MAX_SUBSCRIBERS_PER_TOPIC: usize = 50;

    #[ink(storage)]
    pub struct EventBusContract {
        /// Admin of the event bus
        admin: AccountId,
        /// List of subscribers per topic
        subscribers: Mapping<Topic, Vec<AccountId>>,
        /// Reentrancy protection guard
        reentrancy_guard: ReentrancyGuard,
    }

    impl From<ReentrancyError> for EventBusError {
        fn from(_: ReentrancyError) -> Self {
            EventBusError::ReentrantCall
        }
    }

    #[ink(event)]
    pub struct EventPublished {
        #[ink(topic)]
        pub topic: Topic,
        pub emitter: AccountId,
        pub timestamp: u64,
    }

    #[ink(event)]
    pub struct Subscribed {
        #[ink(topic)]
        pub topic: Topic,
        pub subscriber: AccountId,
    }

    #[ink(event)]
    pub struct Unsubscribed {
        #[ink(topic)]
        pub topic: Topic,
        pub subscriber: AccountId,
    }

    impl EventBusContract {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                admin: Self::env().caller(),
                subscribers: Mapping::default(),
                reentrancy_guard: ReentrancyGuard::new(),
            }
        }
    }

    impl EventBus for EventBusContract {
        #[ink(message)]
        fn publish(
            &mut self,
            topic: Topic,
            mut payload: EventPayload,
        ) -> Result<(), EventBusError> {
            non_reentrant!(self, {
                // Overwrite emitter to ensure authenticity of the payload
                payload.emitter = self.env().caller();

                let subscribers = self.subscribers.get(topic).unwrap_or_default();

                // Loop through each subscriber and deliver the event
                for subscriber_account in &subscribers {
                    // Call the `on_event_received` method of the subscriber
                    // Note: We use try_call or just instantiate the Ref.
                    // Using builder pattern for safety in ink! 4+
                    let mut subscriber: EventSubscriberRef =
                        ink::env::call::FromAccountId::from_account_id(*subscriber_account);

                    // Fire and forget, or handle errors?
                    // If we unwrap, one failing subscriber bricks the entire publish.
                    // We will ignore errors from subscribers to prevent griefing attacks.
                    let _ = subscriber.on_event_received(topic, payload.clone());
                }

                self.env().emit_event(EventPublished {
                    topic,
                    emitter: payload.emitter,
                    timestamp: payload.timestamp,
                });

                Ok(())
            })
        }

        #[ink(message)]
        fn subscribe(&mut self, topic: Topic) -> Result<(), EventBusError> {
            let caller = self.env().caller();
            let mut subs = self.subscribers.get(topic).unwrap_or_default();

            if subs.contains(&caller) {
                return Err(EventBusError::AlreadySubscribed);
            }

            if subs.len() >= MAX_SUBSCRIBERS_PER_TOPIC {
                return Err(EventBusError::MaxSubscribersReached);
            }

            subs.push(caller);
            self.subscribers.insert(topic, &subs);

            self.env().emit_event(Subscribed {
                topic,
                subscriber: caller,
            });

            Ok(())
        }

        #[ink(message)]
        fn unsubscribe(&mut self, topic: Topic) -> Result<(), EventBusError> {
            let caller = self.env().caller();
            let mut subs = self.subscribers.get(topic).unwrap_or_default();

            if let Some(pos) = subs.iter().position(|&x| x == caller) {
                subs.swap_remove(pos);
                self.subscribers.insert(topic, &subs);

                self.env().emit_event(Unsubscribed {
                    topic,
                    subscriber: caller,
                });

                Ok(())
            } else {
                Err(EventBusError::NotSubscribed)
            }
        }

        #[ink(message)]
        fn get_subscribers(&self, topic: Topic) -> Vec<AccountId> {
            self.subscribers.get(topic).unwrap_or_default()
        }
    }
}
