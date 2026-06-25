use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Error {
    ZeroAmount = 1,
    NoIncomeAvailable = 2,
}