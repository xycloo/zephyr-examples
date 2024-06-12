use serde::{Deserialize, Serialize};
use zephyr_sdk::{prelude::*, EnvClient, DatabaseDerive};

#[derive(DatabaseDerive, Clone)]
#[with_name("borrowed")]
#[external("8")]
pub struct Borrowed {
    pub id: i64,
    pub timestamp: u64,
    pub ledger: u32,
    pub pool: String,
    pub asset: String,
    pub borrowed: i128,
    pub delta: i128,
    pub source: String,
}

#[derive(Deserialize)]
pub struct Request {
    pool: String,
}

#[derive(Serialize)]
pub struct ResponseObject {
    pub timestamp: u64,
    pub ledger: u32,
    pub asset: String,
    pub borrowed: String,
    pub delta: String,
    pub source: String,
}

#[no_mangle]
pub extern "C" fn get_borrowed_by_pool() {
    let env = EnvClient::empty();
    let request: Request = env.read_request_body();
    let borrowed: Vec<Borrowed> = env.read_filter().column_equal_to("pool", request.pool).read().unwrap();
    let borrowed: Vec<ResponseObject> = borrowed.iter().map(|obj| {
        ResponseObject {
            timestamp: obj.timestamp,
            ledger: obj.ledger,
            asset: obj.asset.clone(),
            borrowed: (obj.borrowed as i64).to_string(),
            delta: (obj.delta as i64).to_string(),
            source: obj.source.clone()
        }
    }).collect();

    env.conclude(&borrowed)
}
