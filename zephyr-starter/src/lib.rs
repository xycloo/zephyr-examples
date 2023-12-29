use rs_zephyr_sdk::{EnvClient};

#[no_mangle]
pub extern "C" fn on_close() {
    let mut env = EnvClient::default();
}
