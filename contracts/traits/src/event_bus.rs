#![allow(clippy::module_name_repetitions)]

use core::fmt;
use ink::prelude::vec::Vec;
use scale::{Decode, Encode};

#[cfg(feature = "std")]
use scale_info::TypeInfo;

use crate::errors::{event_bus_codes, ContractError, ErrorCategory};

/// A standardized topic identifier for routing events.
pub type Topic = ink::primitives::Hash;

/// The generic payload of an event.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
#[cfg_attr(feature = "std", derive(TypeInfo, ink::storage::traits::StorageLayout))]
pub struct EventPayload {
    pub emitter: ink::primitives::AccountId,
    pub timestamp: u64,
    pub data: Vec<u8>, // SCALE-encoded domain-specific event data
}

/// Errors that can occur within the EventBus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode)]
#[cfg_attr(feature = "std", derive(TypeInfo))]
pub enum EventBusError {
    Unauthorized,
    TopicNotFound,
    AlreadySubscribed,
    NotSubscribed,
    MaxSubscribersReached,
    SubscriberCallFailed,
}

impl fmt::Display for EventBusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EventBusError::Unauthorized => write!(f, "Caller is not authorized"),
            EventBusError::TopicNotFound => write!(f, "The specified topic does not exist"),
            EventBusError::AlreadySubscribed => {
                write!(f, "The caller is already subscribed to this topic")
            }
            EventBusError::NotSubscribed => write!(f, "The caller is not subscribed to this topic"),
            EventBusError::MaxSubscribersReached => {
                write!(f, "The topic has reached the maximum number of subscribers")
            }
            EventBusError::SubscriberCallFailed => {
                write!(f, "Failed to deliver event to one or more subscribers")
            }
        }
    }
}

impl ContractError for EventBusError {
    fn error_code(&self) -> u32 {
        match self {
            EventBusError::Unauthorized => event_bus_codes::EVENT_BUS_UNAUTHORIZED,
            EventBusError::TopicNotFound => event_bus_codes::EVENT_BUS_TOPIC_NOT_FOUND,
            EventBusError::AlreadySubscribed => event_bus_codes::EVENT_BUS_ALREADY_SUBSCRIBED,
            EventBusError::NotSubscribed => event_bus_codes::EVENT_BUS_NOT_SUBSCRIBED,
            EventBusError::MaxSubscribersReached => {
                event_bus_codes::EVENT_BUS_MAX_SUBSCRIBERS_REACHED
            }
            EventBusError::SubscriberCallFailed => {
                event_bus_codes::EVENT_BUS_SUBSCRIBER_CALL_FAILED
            }
        }
    }

    fn error_description(&self) -> &'static str {
        match self {
            EventBusError::Unauthorized => "Caller does not have permission",
            EventBusError::TopicNotFound => {
                "The specified topic does not exist or has no subscribers"
            }
            EventBusError::AlreadySubscribed => "The caller is already a subscriber of this topic",
            EventBusError::NotSubscribed => "The caller is not a subscriber of this topic",
            EventBusError::MaxSubscribersReached => "Cannot add more subscribers to this topic",
            EventBusError::SubscriberCallFailed => "Event delivery to a subscriber failed",
        }
    }

    fn error_category(&self) -> ErrorCategory {
        ErrorCategory::EventBus
    }

    fn error_i18n_key(&self) -> &'static str {
        match self {
            EventBusError::Unauthorized => "event_bus.unauthorized",
            EventBusError::TopicNotFound => "event_bus.topic_not_found",
            EventBusError::AlreadySubscribed => "event_bus.already_subscribed",
            EventBusError::NotSubscribed => "event_bus.not_subscribed",
            EventBusError::MaxSubscribersReached => "event_bus.max_subscribers_reached",
            EventBusError::SubscriberCallFailed => "event_bus.subscriber_call_failed",
        }
    }
}

/// Interface for the central Event Bus contract.
#[ink::trait_definition]
pub trait EventBus {
    /// Publish an event to a specific topic.
    ///
    /// The payload's emitter will be verified or overwritten by the EventBus to be the caller.
    #[ink(message)]
    fn publish(&mut self, topic: Topic, payload: EventPayload) -> Result<(), EventBusError>;

    /// Subscribe the calling contract to a specific topic.
    #[ink(message)]
    fn subscribe(&mut self, topic: Topic) -> Result<(), EventBusError>;

    /// Unsubscribe the calling contract from a specific topic.
    #[ink(message)]
    fn unsubscribe(&mut self, topic: Topic) -> Result<(), EventBusError>;

    /// Get the list of subscribers for a topic
    #[ink(message)]
    fn get_subscribers(&self, topic: Topic) -> Vec<ink::primitives::AccountId>;
}

/// Errors that can occur within the EventSubscriber.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode)]
#[cfg_attr(feature = "std", derive(TypeInfo))]
pub enum EventSubscriberError {
    UnauthorizedSender,
    ProcessingFailed,
}

impl fmt::Display for EventSubscriberError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EventSubscriberError::UnauthorizedSender => {
                write!(f, "Caller is not the authorized EventBus")
            }
            EventSubscriberError::ProcessingFailed => {
                write!(f, "Failed to process the received event")
            }
        }
    }
}

/// Interface that any subscribing contract MUST implement to receive events.
#[ink::trait_definition]
pub trait EventSubscriber {
    /// Callback triggered by the EventBus when a subscribed event is published.
    #[ink(message)]
    fn on_event_received(
        &mut self,
        topic: Topic,
        payload: EventPayload,
    ) -> Result<(), EventSubscriberError>;
}
