use serde::{Deserialize, Serialize};
use zephyr_sdk::{prelude::*, DatabaseDerive, EnvClient};

#[derive(DatabaseDerive, Serialize, Deserialize)]
#[with_name("testt")]
pub struct TableTest {
    numberd: u32,
    other: String
}

#[no_mangle]
pub extern "C" fn on_close() {
    let env = EnvClient::new();

    TableTest {
        numberd: 3,
        other: "testing".into()
    }.put(&env)
}            

#[no_mangle]
pub extern "C" fn test_custom() {
    let env = EnvClient::empty();
    let tables: Vec<TableTest> = env.read();
    env.conclude(&tables);
}
