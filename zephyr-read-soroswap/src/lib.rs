use serde::{Deserialize, Serialize};
use zephyr_sdk::{prelude::*, soroban_sdk::{xdr::{ScString, ScVal}, Address, String as SorobanString}, DatabaseDerive, EnvClient,};

#[derive(DatabaseDerive, Clone)]
#[with_name("pairs")]
#[external("9")]
struct PairsTable {
    address: ScVal,
    token_a: ScVal,
    token_b: ScVal,
    reserve_a: ScVal,
    reserve_b: ScVal,
}

#[derive(Serialize, Deserialize)]
pub struct ResponseObject {
    address: String,
    token_a: String,
    token_b: String,
    reserve_a: i64,
    reserve_b: i64,
}

fn soroban_string_to_string(env: &EnvClient, string: SorobanString) -> String {
    let sc_val: ScVal = env.to_scval(string);
    if let ScVal::String(ScString(s)) = sc_val {
        let s = s.to_utf8_string().unwrap();
        let parts: Vec<&str> = s.split(':').collect();
        parts[0].into()
    } else {
        panic!("value is not a string");
    }
}


// Currently, this function does nothing.
#[no_mangle]
pub extern "C" fn on_close() {
}            

#[no_mangle]
pub extern "C" fn get_all_pairs() {
    let env = EnvClient::empty();
    let all_pairs: Vec<PairsTable> = env.read();
    let converted: Vec<ResponseObject> = all_pairs.iter().map(|row| {
        let address = soroban_string_to_string(&env, env.from_scval::<Address>(&row.address).to_string());
        let token_a = soroban_string_to_string(&env, env.from_scval::<Address>(&row.token_a).to_string());
        let token_b = soroban_string_to_string(&env, env.from_scval::<Address>(&row.token_b).to_string());
        let reserve_a = if let Ok(i) = env.try_from_scval::<i32>(&row.reserve_a) {
            i as i64
        } else {
            env.from_scval::<i128>(&row.reserve_a) as i64
        };

        let reserve_b = if let Ok(i) = env.try_from_scval::<i32>(&row.reserve_b) {
            i as i64
        } else {
            env.from_scval::<i128>(&row.reserve_b) as i64
        };

        ResponseObject {
            address,
            token_a,
            token_b,
            reserve_a,
            reserve_b
        }
    }).collect();
    env.log().debug("Converted all", None);
    env.conclude(converted)
}
