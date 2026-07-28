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
}
