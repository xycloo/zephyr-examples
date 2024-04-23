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
    
    env.log().error("Test Error", None);
    env.log().debug("Test Error", None);
    env.log().warning("Test Error", None);
}
