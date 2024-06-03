use zephyr_sdk::{prelude::*, soroban_sdk::xdr::ScVal, utils, DatabaseDerive, EnvClient};

#[derive(DatabaseDerive, Clone)]
#[with_name("clateral")]
#[external("7")]
pub struct Collateral {
    pub ledger: ScVal,
    pub pool: ScVal,
    pub asset: ScVal,
    pub clateral: ScVal,
}

#[no_mangle]
pub extern "C" fn on_close() {
    let env = EnvClient::new();
}

#[no_mangle]
pub fn dashboard() {
    let env = EnvClient::empty();
    let collaterals: Vec<Collateral> = env.read();
    let amounts: Vec<i64> = collaterals.iter().map(|obj| {
        let ScVal::I128(parts) = obj.clateral.clone() else {panic!()};
        utils::parts_to_i128(&parts) as i64
    }).collect();

    env.conclude(amounts)
}
