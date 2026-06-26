#[cfg(test)]
mod snapshot_voting_tests {
    #[derive(Clone)]
    struct VoterSnapshot { block: u64, power: u128 }

    fn power_at(snapshots: &[VoterSnapshot], proposal_block: u64) -> u128 {
        snapshots.iter()
            .filter(|s| s.block <= proposal_block)
            .map(|s| s.power)
            .last()
            .unwrap_or(0)
    }

    #[test]
    fn ignores_post_proposal_transfers() {
        let snaps = vec![
            VoterSnapshot { block: 10, power: 500 },
            VoterSnapshot { block: 20, power: 1000 },
        ];
        assert_eq!(power_at(&snaps, 15), 500);
    }

    #[test]
    fn uses_latest_pre_proposal_value() {
        let snaps = vec![
            VoterSnapshot { block: 5, power: 200 },
            VoterSnapshot { block: 10, power: 800 },
        ];
        assert_eq!(power_at(&snaps, 10), 800);
    }

    #[test]
    fn no_snapshot_before_proposal_returns_zero() {
        let snaps = vec![VoterSnapshot { block: 50, power: 500 }];
        assert_eq!(power_at(&snaps, 30), 0);
    }

    #[test]
    fn staking_event_after_proposal_excluded() {
        let snaps = vec![
            VoterSnapshot { block: 100, power: 300 },
            VoterSnapshot { block: 200, power: 900 },
        ];
        assert_eq!(power_at(&snaps, 150), 300);
    }
}