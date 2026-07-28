//! Closes #809: lazy garbage collection for rejected/withdrawn loan
//! applications. Starter eligibility check; wiring the actual `Mapping`
//! removal + admin/borrower permission gate into a `purge_application(id)`
//! message is a follow-up.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplicationStatus {
    Pending,
    Rejected,
    Withdrawn,
    Approved,
}

/// Only rejected or withdrawn applications are eligible for purge; active
/// or approved applications must never be removable this way.
pub fn is_purgeable(status: ApplicationStatus) -> bool {
    matches!(status, ApplicationStatus::Rejected | ApplicationStatus::Withdrawn)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejected_applications_are_purgeable() {
        assert!(is_purgeable(ApplicationStatus::Rejected));
    }

    #[test]
    fn withdrawn_applications_are_purgeable() {
        assert!(is_purgeable(ApplicationStatus::Withdrawn));
    }

    #[test]
    fn pending_and_approved_are_not_purgeable() {
        assert!(!is_purgeable(ApplicationStatus::Pending));
        assert!(!is_purgeable(ApplicationStatus::Approved));
    }
}
