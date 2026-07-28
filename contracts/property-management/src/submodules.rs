// SPDX-License-Identifier: MIT
pub mod property_registry {
    pub fn is_registered(id: u64) -> bool {
        id > 0
    }
}

pub mod property_maintenance {
    pub fn schedule_inspection(id: u64) -> bool {
        id > 0
    }
}
