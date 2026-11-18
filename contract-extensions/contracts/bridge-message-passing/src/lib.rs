#![cfg_attr(not(feature = "std"), no_std, no_main)]
#![cfg_attr(test, allow(unused_imports, dead_code))]
#![allow(clippy::new_without_default)]

//! # Bridge message passing (#1437)
//!
//! Lets a contract on one chain call a contract on another. The same contract is
//! deployed on both chains:
//!
//! * **Source chain** - [`BridgeMessagePassing::send_message`] records an
//!   outbound message and returns its **message id**
//!   `blake2x256(SCALE(source_chain, nonce, target, payload))`, emitting
//!   `MessageSent` for relayers. The per-contract `nonce` increases for every
//!   message, so no two messages ever share an id.
//! * **Destination chain** - validators call
//!   [`BridgeMessagePassing::submit_message_attestation`] with the id; once
//!   `threshold` distinct validators attested, anyone can call
//!   [`BridgeMessagePassing::execute_message`] with the full message. The
//!   contract recomputes the id from the supplied fields, so a relayer cannot
//!   substitute a different payload or target.
//!
//! ## Payload
//!
//! `payload` is the SCALE-encoded input of the target message: the 4-byte
//! selector followed by the encoded arguments. It is forwarded unchanged.
//!
//! ## Replay, gas and failures
//!
//! * A message id executes **once**: `Executed` is terminal and re-submitting the
//!   same message fails with `AlreadyExecuted`.
//! * Execution uses at most `gas_limit` (reference time) per message, set by the
//!   admin.
//! * If the target call fails the message becomes `Failed` (event
//!   `MessageFailed`); the admin or the original relayer can `retry_message` (up
//!   to `MAX_RETRIES` times) or `cancel_message`.
//!
//! ## Trust model
//!
//! ink! authenticates the *caller*, so a validator's attestation is the call from
//! the validator's account. The `signature` argument is recorded for auditing but
//! is **not** cryptographically re-verified, and the contract cannot prove that
//! the message was really sent on the source chain; it trusts the validator
//! quorum for that.

#[ink::contract]
pub mod bridge_message_passing {
    use ink::env::call::{build_call, ExecutionInput, Selector};
    use ink::env::hash::Blake2x256;
    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;

    pub const MAX_RETRIES: u8 = 3;
    pub const MAX_PAYLOAD_BYTES: usize = 4_096;
    pub const MAX_VALIDATORS: usize = 32;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub enum MessageStatus {
        /// Collecting attestations / not yet executed.
        Pending,
        Executed,
        Failed,
        Cancelled,
    }

    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub struct OutboundMessage {
        pub nonce: u64,
        pub sender: AccountId,
        pub destination_chain: u64,
        pub target: AccountId,
        pub payload_hash: Hash,
    }

    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub struct InboundMessage {
        pub status: MessageStatus,
        pub attestations: Vec<AccountId>,
        pub retries: u8,
        /// Who ran the failed execution (and may retry it) besides the admin.
        pub relayer: Option<AccountId>,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        Unauthorized,
        PayloadTooLarge,
        EmptyPayload,
        NotAValidator,
        AlreadyAttested,
        ThresholdNotMet,
        /// The supplied message fields do not hash to the given message id.
        MessageIdMismatch,
        AlreadyExecuted,
        MessageNotFound,
        /// The message is not in a state that allows this action.
        InvalidStatus,
        RetryLimitReached,
        InvalidConfig,
        ValidatorLimit,
    }

    #[ink(event)]
    pub struct MessageSent {
        #[ink(topic)]
        pub message_id: Hash,
        pub nonce: u64,
        pub sender: AccountId,
        pub destination_chain: u64,
        pub target: AccountId,
    }

    #[ink(event)]
    pub struct MessageAttested {
        #[ink(topic)]
        pub message_id: Hash,
        pub validator: AccountId,
        pub attestations: u32,
    }

    #[ink(event)]
    pub struct MessageExecuted {
        #[ink(topic)]
        pub message_id: Hash,
        pub target: AccountId,
    }

    #[ink(event)]
    pub struct MessageFailed {
        #[ink(topic)]
        pub message_id: Hash,
        pub retries: u8,
    }

    #[ink(storage)]
    pub struct BridgeMessagePassing {
        admin: AccountId,
        local_chain_id: u64,
        validators: Vec<AccountId>,
        threshold: u32,
        gas_limit: u64,
        next_nonce: u64,
        outbound: Mapping<Hash, OutboundMessage>,
        inbound: Mapping<Hash, InboundMessage>,
    }

    /// Raw bytes with no length prefix, for forwarding pre-encoded arguments.
    struct RawArgs(Vec<u8>);

    impl scale::Encode for RawArgs {
        fn encode_to<T: scale::Output + ?Sized>(&self, dest: &mut T) {
            dest.write(&self.0);
        }
    }

    impl BridgeMessagePassing {
        #[ink(constructor)]
        pub fn new(local_chain_id: u64, threshold: u32, gas_limit: u64) -> Self {
            assert!(threshold > 0 && gas_limit > 0);
            Self {
                admin: Self::env().caller(),
                local_chain_id,
                validators: Vec::new(),
                threshold,
                gas_limit,
                next_nonce: 0,
                outbound: Mapping::default(),
                inbound: Mapping::default(),
            }
        }

        fn ensure_admin(&self) -> Result<(), Error> {
            if self.env().caller() != self.admin {
                return Err(Error::Unauthorized);
            }
            Ok(())
        }

        // ── Administration ─────────────────────────────────────────────

        #[ink(message)]
        pub fn add_validator(&mut self, validator: AccountId) -> Result<(), Error> {
            self.ensure_admin()?;
            if self.validators.contains(&validator) {
                return Ok(());
            }
            if self.validators.len() >= MAX_VALIDATORS {
                return Err(Error::ValidatorLimit);
            }
            self.validators.push(validator);
            Ok(())
        }

        #[ink(message)]
        pub fn remove_validator(&mut self, validator: AccountId) -> Result<(), Error> {
            self.ensure_admin()?;
            self.validators.retain(|v| *v != validator);
            Ok(())
        }

        #[ink(message)]
        pub fn set_threshold(&mut self, threshold: u32) -> Result<(), Error> {
            self.ensure_admin()?;
            if threshold == 0 {
                return Err(Error::InvalidConfig);
            }
            self.threshold = threshold;
            Ok(())
        }

        /// Per-message execution budget (reference time). Admin only.
        #[ink(message)]
        pub fn set_gas_limit(&mut self, gas_limit: u64) -> Result<(), Error> {
            self.ensure_admin()?;
            if gas_limit == 0 {
                return Err(Error::InvalidConfig);
            }
            self.gas_limit = gas_limit;
            Ok(())
        }

        // ── Queries ────────────────────────────────────────────────────

        #[ink(message)]
        pub fn get_validators(&self) -> Vec<AccountId> {
            self.validators.clone()
        }

        #[ink(message)]
        pub fn get_threshold(&self) -> u32 {
            self.threshold
        }

        #[ink(message)]
        pub fn get_gas_limit(&self) -> u64 {
            self.gas_limit
        }

        #[ink(message)]
        pub fn get_outbound(&self, message_id: Hash) -> Option<OutboundMessage> {
            self.outbound.get(message_id)
        }

        #[ink(message)]
        pub fn get_inbound(&self, message_id: Hash) -> Option<InboundMessage> {
            self.inbound.get(message_id)
        }

        /// The id a message `(source_chain, nonce, target, payload)` has.
        #[ink(message)]
        pub fn compute_message_id(&self, source_chain: u64, nonce: u64, target: AccountId, payload: Vec<u8>) -> Hash {
            let mut out = <Blake2x256 as ink::env::hash::HashOutput>::Type::default();
            ink::env::hash_encoded::<Blake2x256, _>(&(source_chain, nonce, target, payload), &mut out);
            Hash::from(out)
        }

        // ── Source chain ───────────────────────────────────────────────

        /// Send `payload` to `target` on `destination_chain`.
        #[ink(message)]
        pub fn send_message(&mut self, destination_chain: u64, target: AccountId, payload: Vec<u8>) -> Result<Hash, Error> {
            if payload.is_empty() {
                return Err(Error::EmptyPayload);
            }
            if payload.len() > MAX_PAYLOAD_BYTES {
                return Err(Error::PayloadTooLarge);
            }
            let nonce = self.next_nonce;
            self.next_nonce += 1;
            let sender = self.env().caller();
            let id = self.compute_message_id(self.local_chain_id, nonce, target, payload.clone());
            let mut payload_hash = <Blake2x256 as ink::env::hash::HashOutput>::Type::default();
            ink::env::hash_bytes::<Blake2x256>(&payload, &mut payload_hash);
            self.outbound.insert(
                id,
                &OutboundMessage { nonce, sender, destination_chain, target, payload_hash: Hash::from(payload_hash) },
            );
            self.env().emit_event(MessageSent { message_id: id, nonce, sender, destination_chain, target });
            Ok(id)
        }

        // ── Destination chain ──────────────────────────────────────────

        /// A validator attests to `message_id`. `signature` is stored by the relayer
        /// network for auditing; it is not re-verified here (see the module docs).
        #[ink(message)]
        pub fn submit_message_attestation(&mut self, message_id: Hash, _signature: [u8; 64]) -> Result<u32, Error> {
            let validator = self.env().caller();
            if !self.validators.contains(&validator) {
                return Err(Error::NotAValidator);
            }
            let mut msg = self.inbound.get(message_id).unwrap_or(InboundMessage {
                status: MessageStatus::Pending,
                attestations: Vec::new(),
                retries: 0,
                relayer: None,
            });
            if msg.status == MessageStatus::Executed || msg.status == MessageStatus::Cancelled {
                return Err(Error::InvalidStatus);
            }
            if msg.attestations.contains(&validator) {
                return Err(Error::AlreadyAttested);
            }
            msg.attestations.push(validator);
            let count = msg.attestations.len() as u32;
            self.inbound.insert(message_id, &msg);
            self.env().emit_event(MessageAttested { message_id, validator, attestations: count });
            Ok(count)
        }

        /// Execute an attested message. Anyone may relay it.
        #[ink(message)]
        pub fn execute_message(
            &mut self,
            message_id: Hash,
            source_chain: u64,
            nonce: u64,
            target: AccountId,
            payload: Vec<u8>,
        ) -> Result<(), Error> {
            if payload.len() < 4 {
                return Err(Error::EmptyPayload);
            }
            if payload.len() > MAX_PAYLOAD_BYTES {
                return Err(Error::PayloadTooLarge);
            }
            if self.compute_message_id(source_chain, nonce, target, payload.clone()) != message_id {
                return Err(Error::MessageIdMismatch);
            }
            let mut msg = self.inbound.get(message_id).ok_or(Error::ThresholdNotMet)?;
            match msg.status {
                MessageStatus::Executed => return Err(Error::AlreadyExecuted),
                MessageStatus::Cancelled | MessageStatus::Failed => return Err(Error::InvalidStatus),
                MessageStatus::Pending => {}
            }
            if (msg.attestations.len() as u32) < self.threshold {
                return Err(Error::ThresholdNotMet);
            }
            msg.relayer = Some(self.env().caller());
            self.run(message_id, target, &payload, &mut msg);
            self.inbound.insert(message_id, &msg);
            Ok(())
        }

        /// Re-run a failed message. Admin or the relayer that executed it. The
        /// original `target` and `payload` must be supplied again (and are checked
        /// against the id).
        #[ink(message)]
        pub fn retry_message(
            &mut self,
            message_id: Hash,
            source_chain: u64,
            nonce: u64,
            target: AccountId,
            payload: Vec<u8>,
        ) -> Result<(), Error> {
            if self.compute_message_id(source_chain, nonce, target, payload.clone()) != message_id {
                return Err(Error::MessageIdMismatch);
            }
            let mut msg = self.inbound.get(message_id).ok_or(Error::MessageNotFound)?;
            self.ensure_admin_or_relayer(&msg)?;
            if msg.status != MessageStatus::Failed {
                return Err(Error::InvalidStatus);
            }
            if msg.retries >= MAX_RETRIES {
                return Err(Error::RetryLimitReached);
            }
            msg.retries += 1;
            self.run(message_id, target, &payload, &mut msg);
            self.inbound.insert(message_id, &msg);
            Ok(())
        }

        /// Give up on a pending or failed message. Admin or its relayer.
        #[ink(message)]
        pub fn cancel_message(&mut self, message_id: Hash) -> Result<(), Error> {
            let mut msg = self.inbound.get(message_id).ok_or(Error::MessageNotFound)?;
            if self.env().caller() != self.admin && !self.is_relayer(&msg) {
                return Err(Error::Unauthorized);
            }
            if msg.status == MessageStatus::Executed || msg.status == MessageStatus::Cancelled {
                return Err(Error::InvalidStatus);
            }
            msg.status = MessageStatus::Cancelled;
            self.inbound.insert(message_id, &msg);
            Ok(())
        }

        // ── Internals ──────────────────────────────────────────────────

        fn is_relayer(&self, msg: &InboundMessage) -> bool {
            msg.relayer == Some(self.env().caller())
        }

        fn ensure_admin_or_relayer(&self, msg: &InboundMessage) -> Result<(), Error> {
            if self.env().caller() == self.admin || self.is_relayer(msg) {
                Ok(())
            } else {
                Err(Error::Unauthorized)
            }
        }

        /// Dispatch and record the outcome on `msg`.
        fn run(&mut self, message_id: Hash, target: AccountId, payload: &[u8], msg: &mut InboundMessage) {
            if self.dispatch(target, payload) {
                msg.status = MessageStatus::Executed;
                self.env().emit_event(MessageExecuted { message_id, target });
            } else {
                msg.status = MessageStatus::Failed;
                self.env().emit_event(MessageFailed { message_id, retries: msg.retries });
            }
        }

        #[cfg(not(test))]
        fn dispatch(&self, target: AccountId, payload: &[u8]) -> bool {
            let mut selector = [0u8; 4];
            selector.copy_from_slice(&payload[..4]);
            build_call::<ink::env::DefaultEnvironment>()
                .call(target)
                .ref_time_limit(self.gas_limit)
                .exec_input(ExecutionInput::new(Selector::new(selector)).push_arg(RawArgs(payload[4..].to_vec())))
                .returns::<()>()
                .try_invoke()
                .map(|r| r.is_ok())
                .unwrap_or(false)
        }

        /// Off-chain tests cannot make cross-contract calls: the outcome is scripted.
        #[cfg(test)]
        fn dispatch(&self, _target: AccountId, _payload: &[u8]) -> bool {
            tests::DISPATCH_OK.with(|c| c.get())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use ink::env::test;
        use std::cell::Cell;

        thread_local! {
            pub static DISPATCH_OK: Cell<bool> = const { Cell::new(true) };
        }

        type Accounts = test::DefaultAccounts<ink::env::DefaultEnvironment>;

        fn dispatch_will(ok: bool) {
            DISPATCH_OK.with(|c| c.set(ok));
        }

        fn setup() -> (BridgeMessagePassing, Accounts) {
            dispatch_will(true);
            let a = test::default_accounts::<ink::env::DefaultEnvironment>();
            test::set_caller::<ink::env::DefaultEnvironment>(a.alice);
            let mut b = BridgeMessagePassing::new(1, 2, 1_000_000);
            b.add_validator(a.bob).unwrap();
            b.add_validator(a.charlie).unwrap();
            b.add_validator(a.django).unwrap();
            (b, a)
        }

        const SIG: [u8; 64] = [7; 64];
        const SOURCE: u64 = 9;

        fn payload() -> Vec<u8> {
            vec![0xDE, 0xAD, 0xBE, 0xEF, 1, 2, 3]
        }

        fn attest(b: &mut BridgeMessagePassing, who: AccountId, id: Hash) -> Result<u32, Error> {
            test::set_caller::<ink::env::DefaultEnvironment>(who);
            b.submit_message_attestation(id, SIG)
        }

        /// A message id as seen on the destination chain, with `n` attestations.
        fn attested(b: &mut BridgeMessagePassing, a: &Accounts, n: usize) -> (Hash, AccountId) {
            let target = a.eve;
            let id = b.compute_message_id(SOURCE, 0, target, payload());
            for who in [a.bob, a.charlie, a.django].into_iter().take(n) {
                attest(b, who, id).unwrap();
            }
            (id, target)
        }

        fn exec(b: &mut BridgeMessagePassing, who: AccountId, id: Hash, target: AccountId) -> Result<(), Error> {
            test::set_caller::<ink::env::DefaultEnvironment>(who);
            b.execute_message(id, SOURCE, 0, target, payload())
        }

        // ── Sending ─────────────────────────────────────────────────────

        #[ink::test]
        fn sending_records_an_outbound_message_with_a_unique_id_per_nonce() {
            let (mut b, a) = setup();
            test::set_caller::<ink::env::DefaultEnvironment>(a.eve);
            let first = b.send_message(2, a.frank, payload()).unwrap();
            let second = b.send_message(2, a.frank, payload()).unwrap();
            assert_ne!(first, second, "the same payload gets a fresh id each time");
            let m = b.get_outbound(first).unwrap();
            assert_eq!((m.nonce, m.sender, m.destination_chain, m.target), (0, a.eve, 2, a.frank));
            assert_eq!(b.get_outbound(second).unwrap().nonce, 1);
            // The id is reproducible from the message fields.
            assert_eq!(first, b.compute_message_id(1, 0, a.frank, payload()));
        }

        #[ink::test]
        fn empty_and_oversized_payloads_are_rejected() {
            let (mut b, a) = setup();
            assert_eq!(b.send_message(2, a.frank, Vec::new()), Err(Error::EmptyPayload));
            assert_eq!(
                b.send_message(2, a.frank, vec![0; MAX_PAYLOAD_BYTES + 1]),
                Err(Error::PayloadTooLarge)
            );
            assert!(b.send_message(2, a.frank, vec![1; MAX_PAYLOAD_BYTES]).is_ok());
        }

        #[ink::test]
        fn sending_emits_a_message_sent_event() {
            let (mut b, a) = setup();
            let before = test::recorded_events().count();
            b.send_message(2, a.frank, payload()).unwrap();
            assert_eq!(test::recorded_events().count() - before, 1);
        }

        // ── Attesting ───────────────────────────────────────────────────

        #[ink::test]
        fn only_validators_attest_and_only_once() {
            let (mut b, a) = setup();
            let id = b.compute_message_id(SOURCE, 0, a.eve, payload());
            assert_eq!(attest(&mut b, a.eve, id), Err(Error::NotAValidator));
            assert_eq!(attest(&mut b, a.bob, id), Ok(1));
            assert_eq!(attest(&mut b, a.bob, id), Err(Error::AlreadyAttested));
            assert_eq!(attest(&mut b, a.charlie, id), Ok(2));
            assert_eq!(b.get_inbound(id).unwrap().attestations, vec![a.bob, a.charlie]);
        }

        #[ink::test]
        fn validator_management_is_admin_only() {
            let (mut b, a) = setup();
            test::set_caller::<ink::env::DefaultEnvironment>(a.bob);
            assert_eq!(b.add_validator(a.eve), Err(Error::Unauthorized));
            assert_eq!(b.remove_validator(a.charlie), Err(Error::Unauthorized));
            assert_eq!(b.set_threshold(1), Err(Error::Unauthorized));
            assert_eq!(b.set_gas_limit(5), Err(Error::Unauthorized));
            test::set_caller::<ink::env::DefaultEnvironment>(a.alice);
            assert_eq!(b.set_threshold(0), Err(Error::InvalidConfig));
            assert_eq!(b.set_gas_limit(0), Err(Error::InvalidConfig));
            b.set_gas_limit(42).unwrap();
            assert_eq!(b.get_gas_limit(), 42);
        }

        // ── Executing ───────────────────────────────────────────────────

        #[ink::test]
        fn execution_needs_the_threshold_of_distinct_attestations() {
            let (mut b, a) = setup();
            let (id, target) = attested(&mut b, &a, 1);
            assert_eq!(exec(&mut b, a.frank, id, target), Err(Error::ThresholdNotMet));
            attest(&mut b, a.charlie, id).unwrap();
            assert_eq!(exec(&mut b, a.frank, id, target), Ok(()));
            assert_eq!(b.get_inbound(id).unwrap().status, MessageStatus::Executed);
        }

        #[ink::test]
        fn an_unattested_message_cannot_be_executed() {
            let (mut b, a) = setup();
            let id = b.compute_message_id(SOURCE, 0, a.eve, payload());
            assert_eq!(exec(&mut b, a.frank, id, a.eve), Err(Error::ThresholdNotMet));
        }

        #[ink::test]
        fn a_relayer_cannot_substitute_the_target_payload_or_nonce() {
            let (mut b, a) = setup();
            let (id, target) = attested(&mut b, &a, 2);
            test::set_caller::<ink::env::DefaultEnvironment>(a.frank);
            assert_eq!(
                b.execute_message(id, SOURCE, 0, a.django, payload()),
                Err(Error::MessageIdMismatch)
            );
            assert_eq!(
                b.execute_message(id, SOURCE, 0, target, vec![0xDE, 0xAD, 0xBE, 0xEF, 9]),
                Err(Error::MessageIdMismatch)
            );
            assert_eq!(
                b.execute_message(id, SOURCE, 1, target, payload()),
                Err(Error::MessageIdMismatch)
            );
            assert_eq!(
                b.execute_message(id, SOURCE + 1, 0, target, payload()),
                Err(Error::MessageIdMismatch)
            );
            assert_eq!(b.get_inbound(id).unwrap().status, MessageStatus::Pending);
        }

        #[ink::test]
        fn a_message_can_only_execute_once() {
            let (mut b, a) = setup();
            let (id, target) = attested(&mut b, &a, 2);
            exec(&mut b, a.frank, id, target).unwrap();
            assert_eq!(exec(&mut b, a.frank, id, target), Err(Error::AlreadyExecuted));
            // Late attestations on an executed message are refused too.
            assert_eq!(attest(&mut b, a.django, id), Err(Error::InvalidStatus));
        }

        #[ink::test]
        fn a_larger_threshold_requires_more_validators() {
            let (mut b, a) = setup();
            b.set_threshold(3).unwrap();
            let (id, target) = attested(&mut b, &a, 2);
            assert_eq!(exec(&mut b, a.frank, id, target), Err(Error::ThresholdNotMet));
            attest(&mut b, a.django, id).unwrap();
            assert!(exec(&mut b, a.frank, id, target).is_ok());
        }

        #[ink::test]
        fn a_removed_validators_earlier_attestation_still_counts_but_they_cannot_attest_again() {
            let (mut b, a) = setup();
            let id = b.compute_message_id(SOURCE, 0, a.eve, payload());
            attest(&mut b, a.bob, id).unwrap();
            test::set_caller::<ink::env::DefaultEnvironment>(a.alice);
            b.remove_validator(a.bob).unwrap();
            assert_eq!(attest(&mut b, a.bob, id), Err(Error::NotAValidator));
            assert_eq!(b.get_inbound(id).unwrap().attestations.len(), 1);
        }

        // ── Failure handling ────────────────────────────────────────────

        #[ink::test]
        fn a_failed_target_call_marks_the_message_failed_and_it_can_be_retried() {
            let (mut b, a) = setup();
            let (id, target) = attested(&mut b, &a, 2);
            dispatch_will(false);
            exec(&mut b, a.frank, id, target).unwrap();
            assert_eq!(b.get_inbound(id).unwrap().status, MessageStatus::Failed);

            // Once the target recovers the relayer (or admin) retries.
            dispatch_will(true);
            test::set_caller::<ink::env::DefaultEnvironment>(a.frank);
            b.retry_message(id, SOURCE, 0, target, payload()).unwrap();
            let m = b.get_inbound(id).unwrap();
            assert_eq!((m.status, m.retries), (MessageStatus::Executed, 1));
        }

        #[ink::test]
        fn retries_are_limited_and_failed_messages_cannot_be_re_executed_directly() {
            let (mut b, a) = setup();
            let (id, target) = attested(&mut b, &a, 2);
            dispatch_will(false);
            exec(&mut b, a.frank, id, target).unwrap();
            assert_eq!(exec(&mut b, a.frank, id, target), Err(Error::InvalidStatus));
            test::set_caller::<ink::env::DefaultEnvironment>(a.alice);
            for _ in 0..MAX_RETRIES {
                b.retry_message(id, SOURCE, 0, target, payload()).unwrap();
            }
            assert_eq!(
                b.retry_message(id, SOURCE, 0, target, payload()),
                Err(Error::RetryLimitReached)
            );
            assert_eq!(b.get_inbound(id).unwrap().retries, MAX_RETRIES);
        }

        #[ink::test]
        fn only_the_admin_or_the_relayer_can_retry_or_cancel() {
            let (mut b, a) = setup();
            let (id, target) = attested(&mut b, &a, 2);
            dispatch_will(false);
            exec(&mut b, a.frank, id, target).unwrap();
            test::set_caller::<ink::env::DefaultEnvironment>(a.django);
            assert_eq!(b.retry_message(id, SOURCE, 0, target, payload()), Err(Error::Unauthorized));
            assert_eq!(b.cancel_message(id), Err(Error::Unauthorized));
            test::set_caller::<ink::env::DefaultEnvironment>(a.frank);
            assert!(b.cancel_message(id).is_ok());
        }

        #[ink::test]
        fn cancelled_messages_stay_cancelled_and_executed_ones_cannot_be_cancelled() {
            let (mut b, a) = setup();
            let (id, target) = attested(&mut b, &a, 2);
            test::set_caller::<ink::env::DefaultEnvironment>(a.alice);
            b.cancel_message(id).unwrap();
            assert_eq!(b.get_inbound(id).unwrap().status, MessageStatus::Cancelled);
            assert_eq!(exec(&mut b, a.frank, id, target), Err(Error::InvalidStatus));
            assert_eq!(b.cancel_message(id), Err(Error::InvalidStatus));

            let id2 = b.compute_message_id(SOURCE, 5, a.eve, payload());
            attest(&mut b, a.bob, id2).unwrap();
            attest(&mut b, a.charlie, id2).unwrap();
            test::set_caller::<ink::env::DefaultEnvironment>(a.frank);
            b.execute_message(id2, SOURCE, 5, a.eve, payload()).unwrap();
            test::set_caller::<ink::env::DefaultEnvironment>(a.alice);
            assert_eq!(b.cancel_message(id2), Err(Error::InvalidStatus));
            assert_eq!(b.cancel_message(Hash::from([9; 32])), Err(Error::MessageNotFound));
        }

        #[ink::test]
        fn execution_and_failure_emit_events() {
            let (mut b, a) = setup();
            let (id, target) = attested(&mut b, &a, 2);
            let before = test::recorded_events().count();
            dispatch_will(false);
            exec(&mut b, a.frank, id, target).unwrap(); // MessageFailed
            assert_eq!(test::recorded_events().count() - before, 1);
            dispatch_will(true);
            test::set_caller::<ink::env::DefaultEnvironment>(a.alice);
            b.retry_message(id, SOURCE, 0, target, payload()).unwrap(); // MessageExecuted
            assert_eq!(test::recorded_events().count() - before, 2);
        }
    }
}
