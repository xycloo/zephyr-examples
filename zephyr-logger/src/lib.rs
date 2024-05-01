use zephyr_sdk::EnvClient;


#[no_mangle]
pub extern "C" fn on_close() {
    let env = EnvClient::new();
    
    env.log().error("Test Error", None);
    env.log().debug("Test Error", None);
    env.log().warning("Test Error", None);
}
