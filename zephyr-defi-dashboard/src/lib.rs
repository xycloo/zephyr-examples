use zephyr_sdk::{prelude::*, soroban_sdk::xdr::{Hash, ScAddress, ScVal}, DatabaseDerive, EnvClient};

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

#[derive(DatabaseDerive, Clone)]
#[with_name("clateral")]
#[external("8")]
pub struct Collateral {
    pub id: i64,
    pub timestamp: u64,
    pub ledger: u32,
    pub pool: String,
    pub asset: String,
    pub clateral: i128,
    pub delta: i128,
    pub source: String,
}

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


#[no_mangle]
pub extern "C" fn dashboard() {
    let env = EnvClient::empty();
    let blend_borrowed: Vec<Borrowed> = env.read();
    let blend_collateral: Vec<Collateral> = env.read();
    let soroswap_pairs: Vec<PairsTable> = env.read();
}

#[test]
fn test() {
    let bytes = stellar_strkey::Contract::from_string("CC7CDFY2VGDODJ7WPO3JIK2MXLOAXL4LRQCC43UJDBAIJ4SVFO3HNPOC").unwrap().0;
    println!("{:?}", ScAddress::Contract(Hash(bytes)).to_xdr_base64(Limits::none()))
}