#[derive(Debug, PartialEq)]
pub enum TreasuryError { NotApproved, ExceedsSpendLimit, InsufficientFunds }

pub struct Treasury { balance: u128, spend_limit: u128 }

impl Treasury {
    pub fn new(balance: u128, spend_limit: u128) -> Self { Self { balance, spend_limit } }
    pub fn deposit(&mut self, amount: u128) { self.balance = self.balance.saturating_add(amount); }
    pub fn balance(&self) -> u128 { self.balance }

    pub fn release(&mut self, approved: bool, amount: u128) -> Result<u128, TreasuryError> {
        if !approved { return Err(TreasuryError::NotApproved); }
        if amount > self.spend_limit { return Err(TreasuryError::ExceedsSpendLimit); }
        if amount > self.balance { return Err(TreasuryError::InsufficientFunds); }
        self.balance = self.balance.saturating_sub(amount);
        Ok(amount)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_within_limit() {
        let mut t = Treasury::new(10_000, 5_000);
        assert_eq!(t.release(true, 3_000).unwrap(), 3_000);
        assert_eq!(t.balance(), 7_000);
    }

    #[test]
    fn exceeds_spend_limit() {
        let mut t = Treasury::new(10_000, 5_000);
        assert_eq!(t.release(true, 6_000), Err(TreasuryError::ExceedsSpendLimit));
    }

    #[test]
    fn unapproved_proposal_rejected() {
        let mut t = Treasury::new(10_000, 5_000);
        assert_eq!(t.release(false, 100), Err(TreasuryError::NotApproved));
    }

    #[test]
    fn insufficient_funds() {
        let mut t = Treasury::new(100, 5_000);
        assert_eq!(t.release(true, 200), Err(TreasuryError::InsufficientFunds));
    }

    #[test]
    fn deposit_increases_balance() {
        let mut t = Treasury::new(0, 1_000);
        t.deposit(500);
        assert_eq!(t.balance(), 500);
    }
}