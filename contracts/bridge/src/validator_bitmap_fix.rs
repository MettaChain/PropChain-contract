pub struct ValidatorBitmapSigner {
    pub max_validators: u32,
}

impl Default for ValidatorBitmapSigner {
    fn default() -> Self {
        Self::new(100)
    }
}

impl ValidatorBitmapSigner {
    pub fn new(max_validators: u32) -> Self {
        Self { max_validators }
    }

    pub fn get_bit_position(&self, validator_index: u32) -> Result<u32, &'static str> {
        if validator_index >= self.max_validators {
            return Err("Validator index out of bounds");
        }
        Ok(validator_index)
    }
}
