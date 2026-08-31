//! Closes #807: move historical collateral assignments off-chain via events
//! instead of an unbounded `collateral_history` Vec. Starter event struct +
//! builder; wiring `env.emit_event(...)` into the assignment path and
//! dropping the Vec field are follow-ups.

pub struct CollateralAssignmentEvent {
    pub property_id: u64,
    pub collateral_id: u64,
    pub assigned_at: u64,
    pub active: bool,
}

/// Builds the event payload for a collateral assignment change, replacing
/// an on-chain `Vec` push with something an off-chain indexer can consume.
pub fn build_assignment_event(
    property_id: u64,
    collateral_id: u64,
    assigned_at: u64,
    active: bool,
) -> CollateralAssignmentEvent {
    CollateralAssignmentEvent { property_id, collateral_id, assigned_at, active }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_an_active_assignment_event() {
        let event = build_assignment_event(1, 2, 1_000, true);
        assert_eq!(event.property_id, 1);
        assert_eq!(event.collateral_id, 2);
        assert!(event.active);
    }

    #[test]
    fn builds_a_deactivation_event() {
        let event = build_assignment_event(1, 2, 2_000, false);
        assert!(!event.active);
    }

    // ── Issue #1037: Additional collateral-event coverage ────────────────────

    /// The `assigned_at` timestamp is stored and readable.
    #[test]
    fn event_preserves_assigned_at_timestamp() {
        let ts = 9_999_999u64;
        let event = build_assignment_event(5, 10, ts, true);
        assert_eq!(event.assigned_at, ts);
    }

    /// All four fields survive the round-trip through `build_assignment_event`.
    #[test]
    fn event_fields_match_inputs_exactly() {
        let event = build_assignment_event(42, 77, 123_456, false);
        assert_eq!(event.property_id, 42);
        assert_eq!(event.collateral_id, 77);
        assert_eq!(event.assigned_at, 123_456);
        assert!(!event.active);
    }

    /// Building multiple events for the same property with different collateral
    /// ids produces independent records (no accidental aliasing).
    #[test]
    fn multiple_events_for_same_property_are_independent() {
        let e1 = build_assignment_event(1, 10, 100, true);
        let e2 = build_assignment_event(1, 20, 200, false);

        assert_eq!(e1.collateral_id, 10);
        assert_eq!(e2.collateral_id, 20);
        assert_ne!(e1.assigned_at, e2.assigned_at);
        assert!(e1.active);
        assert!(!e2.active);
    }

    /// Transitioning the same (property, collateral) pair from active to
    /// inactive is expressed by two separate event payloads; neither mutates
    /// the other.
    #[test]
    fn activation_then_deactivation_events_are_independent() {
        let activated = build_assignment_event(3, 7, 500, true);
        let deactivated = build_assignment_event(3, 7, 600, false);

        assert!(activated.active);
        assert!(!deactivated.active);
        // The timestamps differ — each event is its own snapshot.
        assert_ne!(activated.assigned_at, deactivated.assigned_at);
    }

    /// `assigned_at = 0` is a valid sentinel value (e.g. genesis or
    /// unset); the builder must not reject it.
    #[test]
    fn zero_assigned_at_is_accepted() {
        let event = build_assignment_event(0, 0, 0, false);
        assert_eq!(event.assigned_at, 0);
        assert_eq!(event.property_id, 0);
        assert_eq!(event.collateral_id, 0);
    }
}
