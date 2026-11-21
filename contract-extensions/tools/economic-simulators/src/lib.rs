//! Economic simulations for governance, insurance, crowdfunding, and
//! prediction markets. These models are analytical tools, not contract code.

mod rng;

pub mod campaign;
pub mod governance;
pub mod insurance;
pub mod prediction_market;

/// Errors caused by invalid model inputs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelError {
    InvalidParameter(&'static str),
    EmptyPopulation,
    ZeroIterations,
}
