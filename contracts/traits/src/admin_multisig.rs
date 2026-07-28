// SPDX-License-Identifier: MIT

/// Trait for multi-signature admin key rotation.
///
/// Requires a configurable number of approvals from registered signers
/// before the admin key rotation is confirmed and takes effect.
pub trait MultiSigAdminRotationTrait {
    /// Confirm a pending key rotation with the given set of `approvals`.
    ///
    /// Returns an error if the approval threshold is not met or the
    /// rotation request is not in the expected state.
    fn confirm_key_rotation(&mut self, approvals: Vec<crate::AccountId>) -> Result<(), &'static str>;
}
