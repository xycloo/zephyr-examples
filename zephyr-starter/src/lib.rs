use zephyr_sdk::{prelude::*, EnvClient};

#[no_mangle]
pub extern "C" fn on_close() {
    let env = EnvClient::new();
}
