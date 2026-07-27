pub mod travel_rule {
    pub fn verify_travel_rule(amount: u128) -> bool {
        amount > 0
    }
}

pub mod freeze {
    pub fn is_frozen(asset_id: u128) -> bool {
        asset_id == 0
    }
}

pub mod validator_bitmap {
    pub fn is_valid_bitmap(bitmap: u128) -> bool {
        bitmap > 0
    }
}
