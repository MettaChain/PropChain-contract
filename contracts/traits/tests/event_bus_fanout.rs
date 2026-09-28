#![allow(unexpected_cfgs)] // ink! macros probe `ink-as-dependency`, which this crate does not declare
#![allow(clippy::clone_on_copy)] // fires inside ink! generated storage code

//! Fan-out coverage for the `event_bus` interface (issue #1210).
//!
//! `tests/observer_tests.rs` was the only suite aimed at event delivery. It was
//! deleted in `61c32dc` for three reasons: it did not parse, it was never a
//! declared `[[test]]` target (the `tests/` package root *is* the tests
//! directory, so cargo cannot auto-discover files there), and it targeted
//! `propchain_traits::observer`, which no longer exists. The traits crate ships
//! `event_bus` instead, and that replacement had no tests of its own — this
//! suite, plus `event_subscriber_callback.rs`, is them.
//!
//! Why here and not in the `tests/` package: `propchain-tests` cannot be built
//! at all on the current base commit (`propchain-insurance` and
//! `propchain-bridge` do not compile — see the PR description), so a suite added
//! there could be neither compiled nor executed. Integration tests in this crate
//! do build and run, and sit next to the interface they exercise.
//!
//! `contracts/traits/src/event_bus.rs` declares `EventBus` with
//! `#[ink::trait_definition]`, which puts a `Self: ContractEnv` bound on it.
//! That bound is why [`reference_bus::ReferenceBus`] below is an ink! contract
//! rather than a plain struct: only an ink! storage type can implement the
//! trait, so a suite that wants to call the real trait methods needs a contract
//! to call them on. (ink! emits one `__ink_generate_metadata` symbol per crate,
//! so the subscriber double lives in a sibling file.)
//!
//! No `EventBus` implementation ships in this workspace — the on-chain bus is a
//! separate contract outside this crate's dependency graph — so these tests
//! establish that (a) the trait is implementable by an ink! contract and (b) the
//! delivery and bookkeeping semantics it documents hold for such an
//! implementation. They are not evidence about a deployed contract; the seam is
//! recorded by [`reference_bus_is_not_a_shipped_contract`].
//!
//! Acceptance criteria covered:
//!   * `publish` reaches every subscriber of the topic, and only that topic;
//!   * the bus, not the payload, decides the emitter;
//!   * publishing with no subscribers is rejected;
//!   * subscribe / unsubscribe / double-subscribe / cap bookkeeping;
//!   * the `EventBusError` taxonomy stays stable for off-chain consumers.

use ink::primitives::AccountId;
use propchain_traits::event_bus::{EventBus, EventBusError, EventPayload, Topic};
use propchain_traits::{ContractError, ErrorCategory};

/// Subscriber ceiling the reference bus enforces per topic. Any finite cap
/// exercises the same `MaxSubscribersReached` path; two keeps the test cheap.
const SUBSCRIBER_CAP: usize = 2;

/// Codes are documented as the 11000-11999 band in
/// `contracts/traits/src/errors.rs`; the off-chain indexer keys off them.
const EVENT_BUS_CODE_RANGE: core::ops::RangeInclusive<u32> = 11_000..=11_999;

/// A reference `EventBus` implementation.
///
/// The unit-test environment cannot perform cross-contract calls, so `publish`
/// routes the event by recording one delivery per subscriber of the topic
/// instead of invoking each subscriber's callback. That records the same
/// fan-out decision: which accounts a topic's publish reaches, in which order.
#[ink::contract]
mod reference_bus {
    use super::*;

    /// A delivery, as routed by `publish`: the recipient, the envelope
    /// timestamp, and the emitter the bus stamped on it.
    pub type Delivery = (AccountId, u64, AccountId);

    #[ink(storage)]
    pub struct ReferenceBus {
        subscribers: Vec<(Topic, AccountId)>,
        delivered: Vec<Delivery>,
    }

    impl ReferenceBus {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                subscribers: Vec::new(),
                delivered: Vec::new(),
            }
        }

        /// Deliveries recorded by `publish`, in the order they were routed.
        #[ink(message)]
        pub fn delivered(&self) -> Vec<Delivery> {
            self.delivered.clone()
        }

        /// Registers a subscriber without going through the `subscribe`
        /// message, for arranging multi-account fan-out in a test.
        #[ink(message)]
        pub fn add_subscriber(&mut self, topic: Topic, subscriber: AccountId) {
            self.subscribers.push((topic, subscriber));
        }
    }

    /// Applies the bus's authority over the envelope's origin: whatever emitter
    /// a payload claims, the bus replaces it with the actual runtime caller.
    fn stamped(payload: EventPayload, emitter: AccountId) -> EventPayload {
        EventPayload { emitter, ..payload }
    }

    impl EventBus for ReferenceBus {
        #[ink(message)]
        fn publish(&mut self, topic: Topic, payload: EventPayload) -> Result<(), EventBusError> {
            let payload = stamped(payload, self.env().caller());

            let recipients: Vec<AccountId> = self
                .subscribers
                .iter()
                .filter(|(subscribed, _)| *subscribed == topic)
                .map(|(_, subscriber)| *subscriber)
                .collect();

            if recipients.is_empty() {
                return Err(EventBusError::TopicNotFound);
            }

            for recipient in recipients {
                self.delivered
                    .push((recipient, payload.timestamp, payload.emitter));
            }
            Ok(())
        }

        #[ink(message)]
        fn subscribe(&mut self, topic: Topic) -> Result<(), EventBusError> {
            let caller = self.env().caller();
            if self
                .subscribers
                .iter()
                .any(|(subscribed, subscriber)| *subscribed == topic && *subscriber == caller)
            {
                return Err(EventBusError::AlreadySubscribed);
            }

            let count = self
                .subscribers
                .iter()
                .filter(|(subscribed, _)| *subscribed == topic)
                .count();
            if count >= super::SUBSCRIBER_CAP {
                return Err(EventBusError::MaxSubscribersReached);
            }

            self.subscribers.push((topic, caller));
            Ok(())
        }

        #[ink(message)]
        fn unsubscribe(&mut self, topic: Topic) -> Result<(), EventBusError> {
            let caller = self.env().caller();
            let position = self
                .subscribers
                .iter()
                .position(|(subscribed, subscriber)| *subscribed == topic && *subscriber == caller);

            let Some(position) = position else {
                return Err(EventBusError::NotSubscribed);
            };

            self.subscribers.remove(position);
            Ok(())
        }

        #[ink(message)]
        fn get_subscribers(&self, topic: Topic) -> Vec<AccountId> {
            self.subscribers
                .iter()
                .filter(|(subscribed, _)| *subscribed == topic)
                .map(|(_, subscriber)| *subscriber)
                .collect()
        }
    }
}

use reference_bus::ReferenceBus;

fn account(seed: u8) -> AccountId {
    AccountId::from([seed; 32])
}

fn topic(seed: u8) -> Topic {
    ink::primitives::Hash::from([seed; 32])
}

fn payload(timestamp: u64, data: Vec<u8>) -> EventPayload {
    EventPayload {
        // Deliberately not the bus caller: `publish` must overwrite this.
        emitter: account(0xEE),
        timestamp,
        data,
    }
}

/// Runs `body` as `caller`, so `self.env().caller()` inside the contract is the
/// account the test intends to act as.
fn as_account<T>(caller: AccountId, body: impl FnOnce() -> T) -> T {
    ink::env::test::set_caller::<ink::env::DefaultEnvironment>(caller);
    body()
}

// ── Documenting the seam ──────────────────────────────────────────────

#[ink::test]
fn reference_bus_is_not_a_shipped_contract() {
    // There is no `EventBus` implementation in the workspace: the traits crate
    // ships the interface, and no crate here depends on a bus contract. This
    // test records the gap in the suite rather than only in a comment; when a
    // real bus lands, the fan-out assertions above should be re-pointed at it
    // and this test deleted.
    let traits_source = include_str!("../src/event_bus.rs");
    assert!(
        !traits_source.contains("impl EventBus for"),
        "an EventBus implementation now exists in the traits crate — \
         re-point the fan-out tests at it and remove this placeholder"
    );
}

// ── Normal path: fan-out ──────────────────────────────────────────────

#[ink::test]
fn publish_reaches_every_subscriber_of_the_topic() {
    let mut bus = ReferenceBus::new();
    let topic_a = topic(0xAA);
    bus.add_subscriber(topic_a, account(2));
    bus.add_subscriber(topic_a, account(3));

    let event = payload(1_000, vec![7, 8, 9]);
    let caller = account(1);
    as_account(caller, || {
        assert_eq!(bus.publish(topic_a, event.clone()), Ok(()));
    });

    // Both subscribers, in subscription order, with the same stamped envelope.
    assert_eq!(
        bus.delivered(),
        vec![
            (account(2), event.timestamp, caller),
            (account(3), event.timestamp, caller),
        ]
    );
}

#[ink::test]
fn publish_does_not_reach_other_topics() {
    let mut bus = ReferenceBus::new();
    bus.add_subscriber(topic(0xAA), account(2));
    bus.add_subscriber(topic(0xBB), account(3));

    let caller = account(1);
    as_account(caller, || {
        assert_eq!(bus.publish(topic(0xAA), payload(1, vec![1])), Ok(()));
    });

    assert_eq!(bus.delivered(), vec![(account(2), 1, caller)]);
}

#[ink::test]
fn publish_overwrites_a_forged_emitter() {
    let mut bus = ReferenceBus::new();
    let topic_a = topic(0xAA);
    bus.add_subscriber(topic_a, account(2));

    // `payload()` stamps emitter = 0xEE. The caller is whatever the runtime
    // reports, and `publish` must stamp that over the payload's own claim.
    let mut forged = payload(1, vec![]);
    forged.emitter = account(0x99);

    let caller = account(1);
    as_account(caller, || {
        assert_eq!(bus.publish(topic_a, forged.clone()), Ok(()));
    });

    assert_eq!(bus.delivered(), vec![(account(2), 1, caller)]);
    assert_ne!(bus.delivered()[0].2, account(0x99));
    assert_ne!(bus.delivered()[0].2, account(0xEE));
}

#[ink::test]
fn every_subscriber_sees_every_publish_in_order() {
    let mut bus = ReferenceBus::new();
    let topic_a = topic(0xAA);
    bus.add_subscriber(topic_a, account(2));
    bus.add_subscriber(topic_a, account(3));

    for timestamp in 1..=3u64 {
        assert_eq!(bus.publish(topic_a, payload(timestamp, vec![])), Ok(()));
    }

    let timestamps: Vec<u64> = bus.delivered().iter().map(|(_, at, _)| *at).collect();
    assert_eq!(timestamps, vec![1, 1, 2, 2, 3, 3]);
}

// ── Failure path: no subscribers ──────────────────────────────────────

#[ink::test]
fn publish_without_subscribers_is_rejected_and_delivers_nothing() {
    let mut bus = ReferenceBus::new();

    assert_eq!(
        bus.publish(topic(0xAA), payload(1, vec![1])),
        Err(EventBusError::TopicNotFound)
    );
    assert!(bus.delivered().is_empty());
}

#[ink::test]
fn publisher_may_be_the_only_subscriber() {
    let mut bus = ReferenceBus::new();
    let topic_a = topic(0xAA);
    let caller = account(1);

    as_account(caller, || {
        assert_eq!(bus.subscribe(topic_a), Ok(()));
        assert_eq!(bus.publish(topic_a, payload(1, vec![2])), Ok(()));
    });
    assert_eq!(bus.delivered(), vec![(caller, 1, caller)]);
}

// ── Subscription bookkeeping ──────────────────────────────────────────

#[ink::test]
fn subscribe_then_unsubscribe_stops_delivery() {
    let mut bus = ReferenceBus::new();
    let topic_a = topic(0xAA);
    let caller = account(1);

    as_account(caller, || {
        assert_eq!(bus.subscribe(topic_a), Ok(()));
        assert_eq!(bus.get_subscribers(topic_a), vec![caller]);

        assert_eq!(bus.unsubscribe(topic_a), Ok(()));
        assert!(bus.get_subscribers(topic_a).is_empty());
        assert_eq!(
            bus.publish(topic_a, payload(1, vec![])),
            Err(EventBusError::TopicNotFound)
        );
    });
}

#[ink::test]
fn double_subscribe_is_rejected() {
    let mut bus = ReferenceBus::new();
    let topic_a = topic(0xAA);

    as_account(account(1), || {
        assert_eq!(bus.subscribe(topic_a), Ok(()));
        assert_eq!(
            bus.subscribe(topic_a),
            Err(EventBusError::AlreadySubscribed)
        );
        assert_eq!(bus.get_subscribers(topic_a).len(), 1);
    });
}

#[ink::test]
fn unsubscribe_without_a_subscription_is_rejected() {
    let mut bus = ReferenceBus::new();

    as_account(account(1), || {
        assert_eq!(
            bus.unsubscribe(topic(0xAA)),
            Err(EventBusError::NotSubscribed)
        );
    });
}

#[ink::test]
fn unsubscribe_only_affects_the_callers_own_subscription() {
    let mut bus = ReferenceBus::new();
    let topic_a = topic(0xAA);
    bus.add_subscriber(topic_a, account(2));

    // account(1) never subscribed to topic_a.
    as_account(account(1), || {
        assert_eq!(bus.unsubscribe(topic_a), Err(EventBusError::NotSubscribed));
    });
    assert_eq!(bus.get_subscribers(topic_a), vec![account(2)]);
}

#[ink::test]
fn subscriber_cap_is_enforced_per_topic() {
    let mut bus = ReferenceBus::new();
    let topic_a = topic(0xAA);
    let topic_b = topic(0xBB);

    as_account(account(2), || assert_eq!(bus.subscribe(topic_a), Ok(())));
    as_account(account(3), || assert_eq!(bus.subscribe(topic_a), Ok(())));
    as_account(account(4), || {
        assert_eq!(
            bus.subscribe(topic_a),
            Err(EventBusError::MaxSubscribersReached)
        );
        // The cap is per topic, and a rejected subscribe adds nobody.
        assert_eq!(bus.subscribe(topic_b), Ok(()));
    });

    assert_eq!(bus.get_subscribers(topic_a), vec![account(2), account(3)]);
    assert_eq!(bus.get_subscribers(topic_b), vec![account(4)]);
}

#[ink::test]
fn subscribers_are_reported_in_subscription_order() {
    let mut bus = ReferenceBus::new();
    let topic_a = topic(0xAA);
    bus.add_subscriber(topic_a, account(2));
    bus.add_subscriber(topic_a, account(3));
    bus.add_subscriber(topic_a, account(4));

    assert_eq!(
        bus.get_subscribers(topic_a),
        vec![account(2), account(3), account(4)]
    );
    assert!(bus.get_subscribers(topic(0xCC)).is_empty());
}

// ── Error taxonomy stability ──────────────────────────────────────────

#[ink::test]
fn error_codes_are_unique_and_inside_the_event_bus_band() {
    let codes: Vec<u32> = all_event_bus_errors()
        .iter()
        .map(|error| error.error_code())
        .collect();

    for code in &codes {
        assert!(
            EVENT_BUS_CODE_RANGE.contains(code),
            "code {code} escaped the documented EventBus range"
        );
        assert_ne!(*code, 0, "0 is the sentinel for 'no error'");
    }

    let mut unique = codes.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(
        unique.len(),
        codes.len(),
        "two EventBus errors share a code: {codes:?}"
    );
}

#[ink::test]
fn every_variant_reports_the_event_bus_category_and_a_keyed_message() {
    for error in all_event_bus_errors() {
        assert_eq!(error.error_category(), ErrorCategory::EventBus);
        assert!(
            !error.error_description().is_empty(),
            "{error:?} has no human-readable description"
        );
        assert!(
            error.error_i18n_key().starts_with("event_bus."),
            "{error:?} i18n key is not namespaced: {}",
            error.error_i18n_key()
        );
    }
}

#[ink::test]
fn error_message_snapshot_matches_the_taxonomy() {
    let error = EventBusError::MaxSubscribersReached;
    let message = error.to_error_message();

    assert_eq!(message.code, error.error_code());
    assert_eq!(message.category, ErrorCategory::EventBus);
    assert_eq!(message.message, error.error_description());
    assert_eq!(message.i18n_key, error.error_i18n_key());
}

fn all_event_bus_errors() -> [EventBusError; 7] {
    [
        EventBusError::Unauthorized,
        EventBusError::TopicNotFound,
        EventBusError::AlreadySubscribed,
        EventBusError::NotSubscribed,
        EventBusError::MaxSubscribersReached,
        EventBusError::SubscriberCallFailed,
        EventBusError::ReentrantCall,
    ]
}
