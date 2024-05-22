use serde::{Deserialize, Serialize};
use zephyr_sdk::{prelude::*, soroban_sdk::{self, contracttype, String as SorString, Address}, EnvClient};

#[contracttype]
pub enum DataKey {
    Balance(Address)
}
#[derive(Serialize, Deserialize)]
pub struct Balance {
    addr: String,
    balance: i128
}
#[derive(Serialize, Deserialize)]
pub struct Request {
    addesses: Vec<String>
}

#[no_mangle]
pub extern "C" fn get_all_balances() {
    let env = EnvClient::empty();
    let req: Request = env.read_request_body();
    let mut balances = Vec::new();
    for addr in req.addesses {
        let source_addr = Address::from_string(&SorString::from_str(&env.soroban(), &addr));
        let res: i128 = env.read_contract_entry_by_key([0;32], DataKey::Balance(source_addr)).unwrap().unwrap();
        balances.push(Balance { addr, balance: res })
    }

    env.conclude(balances)
}     
