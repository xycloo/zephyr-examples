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
    let z_env = EnvClient::new();
    let env = rs_zephyr_sdk::soroban_sdk::Env::default();
/* 
    let reader = z_env.reader();
    for event in reader.soroban_events() {
        if let ContractEventBody::V0(v0) = event.body {
            if v0.topics[0].try_into_val(&env).unwrap() == Symbol::new(&env, "transfer") {

            }
        }
    }*/

    unsafe {
        log(9);
    }
    z_env.send_web_request(AgnosticRequest {
        body: Some("Hello from Zephyr Monitor Program!".into()),
        url: "https://tdep.requestcatcher.com/".into(),
        method: rs_zephyr_sdk::Method::Get,
        headers: vec![("Custom".into(), "Header".into())]
    })
}
