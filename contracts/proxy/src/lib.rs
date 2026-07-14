#![cfg_attr(not(feature = "std"), no_std)]

// Minimal stub for propchain-proxy. The original transparent-proxy-with-upgrade-governance
// implementation was too broken to surgically fix after cascade deletions (round 33).
// Replaced with an empty contract that still compiles as a workspace member.
// See docs/REGENERATE_PROXY.md (TODO) for the planned re-implementation.

#[ink::contract]
pub mod propchain_proxy {
    #[ink(storage)]
    pub struct TransparentProxy {}

    impl TransparentProxy {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {}
        }

        #[ink(message)]
        pub fn noop(&self) {}
    }
}
