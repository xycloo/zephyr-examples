use rs_zephyr_sdk::{bincode, log, soroban_sdk::{Env, Symbol, TryFromVal, TryIntoVal, Val}, stellar_xdr::next::{ContractEventBody, Limits, WriteXdr}, AgnosticRequest, Condition, DatabaseDerive, DatabaseInteract, EnvClient, ZephyrVal};


fn into_val<T: TryFromVal<Env, Val>>(env: &Env, val: &Val) -> Option<T> {
    if let Ok(v) = T::try_from_val(env, val) {
        Some(v)
    } else {
        None
    }
}

#[no_mangle]
pub extern "C" fn on_close() {
    let env = EnvClient::new();


    z_env.send_web_request(AgnosticRequest {
        body: Some("Hello from Zephyr Monitor Program!".into()),
        url: "https://tdep.requestcatcher.com/".into(),
        method: rs_zephyr_sdk::Method::Get,
        headers: vec![("Custom".into(), "Header".into())]
    })
}
