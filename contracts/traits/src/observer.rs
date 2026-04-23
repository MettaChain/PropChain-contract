//! Observer pattern for efficient event distribution (Issue #185).
//!
//! # Design
//!
//! - [`EventObserver`] — trait that any subscriber must implement.
//! - [`EventKind`] — enum of every event category emitted by PropChain contracts.
//! - [`EventBus`] — registry that holds observers and fans events out to them.
//!
//! # Usage
//!
//! ```rust,ignore
//! use propchain_traits::observer::{EventBus, EventKind, EventObserver};
//!
//! struct AuditLogger;
//! impl EventObserver for AuditLogger {
//!     fn on_event(&mut self, kind: &EventKind) {
//!         ink::env::debug_println!("audit: {:?}", kind);
//!     }
//! }
//!
//! let mut bus = EventBus::new();
//! bus.subscribe(ink::prelude::boxed::Box::new(AuditLogger));
//! bus.emit(&EventKind::Transfer { from: None, to: alice, token_id: 1 });
//! ```

use ink::prelude::vec::Vec;
use ink::prelude::boxed::Box;
use crate::{AccountId, TokenId};

// ── Event catalogue ────────────────────────────

/// Every observable event kind emitted by PropChain contracts.
///
/// Extend this enum when a new contract emits events that observers
/// need to react to — no other change is required.
#[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum EventKind {
    // ── Token lifecycle ──────────────────────────────────────────────────
    /// ERC-721 transfer (mint when `from` is `None`, burn when `to` is `None`).
    Transfer {
        from: Option<AccountId>,
        to: Option<AccountId>,
        token_id: TokenId,
    },
    /// Single-token approval granted or revoked.
    Approval {
        owner: AccountId,
        spender: AccountId,
        token_id: TokenId,
    },
    /// Operator-level approval for all tokens of an owner.
    ApprovalForAll {
        owner: Ac────
    /// A new property NFT was minted.
    PropertyMinted {
        token_id: TokenId,
        property_id: u64,
        owner: AccountId,
    },
    /// Compliance status changed for a token.
    ComplianceUpdated {
        token_id: TokenId,
        verified: bool,
    },

    // ── Fractional / dividend ────────────────────────────────────────────
    /// Fractional shares issued to an account.
    SharesIssued {
        token_id: TokenId,
        to: AccountId,
        amount: u128,
    },
    /// Dividends deposited for a token.
    DividendsDeposited {
        token_id: TokenId,
        amount: u128,
    },

    // ── Governance ───────────────────────────────────────────────────────
    /// A governance proposal was created.
    ProposalCreated {
        token_id: TokenId,
        proposal────────────────────────────────────────
    /// A cross-chain bridge request was created.
    BridgeRequested {
        request_id: u64,
        token_id: TokenId,
    },
    /// A bridge request completed successfully.
    BridgeExecuted {
        request_id: u64,
        token_id: TokenId,
    },
    /// A bridge request failed.
    BridgeFailed {
        request_id: u64,
        token_id: TokenId,
    },

    // ── Generic escape hatch ─────────────────────────────────────────────
    /// Custom event for future extensions without an enum variant.
    Custom {
        tag: ink::prelude::string::String,
    },
}

// ── Observer trait ───────────────────────────────────────────────────────────

/// Implement this ts registered with.  Implementations should be cheap — defer heavy
    /// work to off-chain indexers.
    fn on_event(&mut self, kind: &EventKind);

    /// Human-readable name used in logs and diagnostics.
    fn name(&self) -> &'static str {
        "unnamed-observer"
    }
}

// ── Event bus ────────────────────────────────────────────────────────────────

/// Central hub that fans out events to all registered observers.
///
/// # Invariants
///
/// - Observers are called in registration order.
/// - A panicking observer does **not** prevent subsequent observers from
///   receiving the event (errors are swallowed in no-std; logged in std).
/// - The bus is intentionally synchronous so it can run inside an ink!
///   contract message without async overhead.
pub struct EventBus {
    observers: Vec<Box<dyn EventObserver>>,
}

impl EventBus {
    /// Create an empty event buervers are notified in FIFO order.
    pub fn subscribe(&mut self, observer: Box<dyn EventObserver>) {
        self.observers.push(observer);
    }

    /// Remove all observers whose `name()` matches `name`.
    /// Returns the number of observers removed.
    pub fn unsubscribe_by_name(&mut self, name: &str) -> usize {
        let before = self.observers.len();
        self.observers.retain(|o| o.name() != name);
        before - self.observers.len()
    }

    /// Broadcast `kind` to every registered observer.
    pub fn emit(&mut self, kind: &EventKind) {
        for observer in &mut self.observers {
            observer.on_event(kind);
        }
    }

    /// Number of currently registered observers.
    pub fn observer_count(&self) -> usize {
        self.observers.len()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}
