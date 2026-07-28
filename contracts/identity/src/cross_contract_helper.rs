// SPDX-License-Identifier: MIT
pub struct CrossContractCaller;

impl CrossContractCaller {
    pub fn call_contract<T, F>(contract_call: F) -> Result<T, &'static str>
    where
        F: FnOnce() -> Result<T, &'static str>,
    {
        contract_call()
    }
}
