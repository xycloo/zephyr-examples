use zephyr_sdk::{prelude::*, EnvClient};

#[no_mangle]
pub extern "C" fn get_contract_spec() {
    let env = EnvClient::empty();
}            
