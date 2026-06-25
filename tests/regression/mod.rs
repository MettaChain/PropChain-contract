/// # Regression Test Suite (Issue #487)
///
/// Each test in this suite references the closed bug it guards against.
/// When a bug is reopened the failing test pinpoints the exact regression.
///
/// Modules:
///   - `reinsurance_stats_derives` — encodes/decodes ReinsuranceStats (fixed in earlier work)
///   - `reentrancy_guard`         — verifies the guard on each protected function
///   - `closed_bugs`              — miscellaneous one-off regressions
pub mod closed_bugs;
pub mod reentrancy_guard;
pub mod reinsurance_stats_derives;
