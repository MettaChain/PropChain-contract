#![allow(unexpected_cfgs)] // ink! macros probe `ink-as-dependency`, which this crate does not declare
#![allow(clippy::clone_on_copy)] // fires inside ink! generated storage code

//! Callback coverage for the `EventSubscriber` half of the `event_bus`
//! interface (issue #1210).
//!
//! Sibling of `event_bus_fanout.rs`, which covers the `EventBus` side; the
//! rationale for the suite as a whole (what replaced the deleted
//! `tests/observer_tests.rs`, and why it lives in this crate) is documented
//! there. The two are separate files because ink! emits one
//! `__ink_generate_metadata` symbol per crate, so a single test binary cannot
//! hold two `#[ink::contract]` modules.
//!
//! `#[ink::trait_definition]` puts a `Self: ContractEnv` bound on
//! `EventSubscriber`, so `on_event_received` can only be exercised through an
//! ink! contract. [`recording_subscriber::RecordingSubscriber`] is that
//! contract; what it records is what a real subscriber would have received.
//!
//! Acceptance criteria covered:
//!   * payloads reach the callback unchanged and survive a SCALE round trip;
//!   * the callback records delivery on the happy path;
//!   * a subscriber that rejects an event reports it and records nothing;
//!   * the `EventSubscriberError` surface stays stable.

use ink::primitives::AccountId;
use propchain_traits::event_bus::{EventPayload, EventSubscriber, EventSubscriberError, Topic};
use scale::{Decode, Encode};

/// An `EventSubscriber` that records what it was handed.
#[ink::contract]
mod recording_subscriber {
    use super::*;

    #[ink(storage)]
    pub struct RecordingSubscriber {
        owner: AccountId,
        /// `(topic, envelope timestamp)` for every event accepted so far.
        seen: Vec<(Topic, u64)>,
        /// When set, every callback fails, modelling a misbehaving subscriber.
        fail: bool,
    }

    impl RecordingSubscriber {
        #[ink(constructor)]
        pub fn new(owner: AccountId, fail: bool) -> Self {
            Self {
                owner,
                seen: Vec::new(),
                fail,
            }
        }

        #[ink(message)]
        pub fn owner(&self) -> AccountId {
            self.owner
        }

        #[ink(message)]
        pub fn seen(&self) -> Vec<(Topic, u64)> {
            self.seen.clone()
        }

        #[ink(message)]
        pub fn seen_count(&self) -> u32 {
            self.seen.len() as u32
        }
    }

    impl EventSubscriber for RecordingSubscriber {
        #[ink(message)]
        fn on_event_received(
            &mut self,
            topic: Topic,
            payload: EventPayload,
        ) -> Result<(), EventSubscriberError> {
            if self.fail {
                return Err(EventSubscriberError::ProcessingFailed);
            }
            // Records the envelope fields a delivery assertion needs. Kept to
            // `(topic, timestamp)` because `EventPayload` is a message argument,
            // not a storage item.
            self.seen.push((topic, payload.timestamp));
            Ok(())
        }
    }
}

use recording_subscriber::RecordingSubscriber;

fn account(seed: u8) -> AccountId {
    AccountId::from([seed; 32])
}

fn topic(seed: u8) -> Topic {
    ink::primitives::Hash::from([seed; 32])
}

fn payload(timestamp: u64, data: Vec<u8>) -> EventPayload {
    EventPayload {
        emitter: account(0xEE),
        timestamp,
        data,
    }
}

#[ink::test]
fn subscriber_records_the_delivered_envelope() {
    let mut subscriber = RecordingSubscriber::new(account(2), false);
    let topic_a = topic(0xAA);
    let event = payload(4_242, vec![0xDE, 0xAD]);

    assert_eq!(subscriber.on_event_received(topic_a, event.clone()), Ok(()));

    assert_eq!(subscriber.seen_count(), 1);
    assert_eq!(subscriber.seen(), vec![(topic_a, event.timestamp)]);
    assert_eq!(subscriber.owner(), account(2));
}

#[ink::test]
fn decoded_payload_reaches_the_subscriber_unchanged() {
    // A subscriber is a separate contract in the real system, so the payload it
    // sees is the one that survived SCALE encoding on the way in.
    let mut subscriber = RecordingSubscriber::new(account(2), false);
    let topic_a = topic(0xAA);

    let event = payload(4_242, vec![0xDE, 0xAD, 0xBE, 0xEF]);
    let decoded = EventPayload::decode(&mut event.encode().as_slice()).expect("payload decodes");
    assert_eq!(decoded, event);

    assert_eq!(
        subscriber.on_event_received(topic_a, decoded.clone()),
        Ok(())
    );
    assert_eq!(subscriber.seen(), vec![(topic_a, decoded.timestamp)]);
}

#[ink::test]
fn empty_and_large_payloads_round_trip_to_the_subscriber() {
    let mut subscriber = RecordingSubscriber::new(account(2), false);
    let topic_a = topic(0xAA);

    let empty = payload(0, vec![]);
    let decoded_empty =
        EventPayload::decode(&mut empty.encode().as_slice()).expect("empty payload decodes");
    assert!(decoded_empty.data.is_empty());
    assert_eq!(subscriber.on_event_received(topic_a, decoded_empty), Ok(()));

    let large = vec![0xAB; 4_096];
    let decoded_large = EventPayload::decode(&mut payload(1, large.clone()).encode().as_slice())
        .expect("large payload decodes");
    assert_eq!(decoded_large.data.len(), 4_096);
    assert_eq!(decoded_large.data, large);
    assert_eq!(subscriber.on_event_received(topic_a, decoded_large), Ok(()));

    assert_eq!(subscriber.seen_count(), 2);
}

#[ink::test]
fn a_failing_subscriber_reports_processing_failure() {
    let mut subscriber = RecordingSubscriber::new(account(3), true);

    assert_eq!(
        subscriber.on_event_received(topic(0xAA), payload(1, vec![5])),
        Err(EventSubscriberError::ProcessingFailed)
    );
    // A rejected event is not recorded as delivered.
    assert_eq!(subscriber.seen_count(), 0);
    assert!(subscriber.seen().is_empty());
}

#[ink::test]
fn a_subscriber_that_rejected_once_accepts_later_events() {
    let mut subscriber = RecordingSubscriber::new(account(3), false);
    let topic_a = topic(0xAA);

    assert_eq!(
        subscriber.on_event_received(topic_a, payload(1, vec![])),
        Ok(())
    );
    assert_eq!(subscriber.seen_count(), 1);
    assert_eq!(
        subscriber.on_event_received(topic_a, payload(2, vec![])),
        Ok(())
    );
    assert_eq!(subscriber.seen_count(), 2);
    assert_eq!(subscriber.seen(), vec![(topic_a, 1), (topic_a, 2)]);
}

#[ink::test]
fn subscriber_error_messages_are_distinct_and_human_readable() {
    let unauthorized = EventSubscriberError::UnauthorizedSender.to_string();
    let failed = EventSubscriberError::ProcessingFailed.to_string();

    assert!(!unauthorized.is_empty());
    assert!(!failed.is_empty());
    assert_ne!(unauthorized, failed);
    assert_eq!(
        EventSubscriberError::UnauthorizedSender,
        EventSubscriberError::UnauthorizedSender
    );
    assert_ne!(
        EventSubscriberError::UnauthorizedSender,
        EventSubscriberError::ProcessingFailed
    );
}
