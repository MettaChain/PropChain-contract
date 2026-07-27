pub struct LiquidationGuard {
    pub locked: bool,
}

impl LiquidationGuard {
    pub fn new() -> Self {
        Self { locked: false }
    }

    pub fn lock(&mut self) -> Result<(), &'static str> {
        if self.locked {
            return Err("ReentrancyGuard: reentrant liquidation call");
        }
        self.locked = true;
        Ok(())
    }

    pub fn unlock(&mut self) {
        self.locked = false;
    }
}
