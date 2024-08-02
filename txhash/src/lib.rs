use zephyr_sdk::{prelude::*, EnvClient};

#[no_mangle]
pub extern "C" fn on_close() {
    let env = EnvClient::new();

    for (event, txhash) in env.reader().pretty().soroban_events_and_txhash() {
        env.log().debug(format!("Got event and hash {:?} {:?}", txhash, event), None)
    }
}
