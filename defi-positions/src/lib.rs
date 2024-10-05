use serde::{Deserialize, Serialize};
use utils::find_address_in_scval;
use zephyr_sdk::{prelude::*, soroban_sdk::{self, contracttype, xdr::ScVal, Address}, EnvClient};

mod utils;

#[derive(Deserialize)]
pub struct Request {
    /// Address to scan.
    address: String,

    /// Additional contracts to scan.
    additional: Option<Vec<(String, String)>>,
}

#[derive(Deserialize, Serialize)]
pub struct Found {
    protocol: String,
    key: ScVal,
    value: ScVal
}

#[contracttype]
enum DataKey {
    Balance(Address)
}

const DEFAULT_SCAN: [(&'static str, &'static str); 5] = [
    // XycLoans
    ("xycloans", "CBV4OSTRMD2IJJYX3XRNIIVCNA5B2ZLHQMUEUJSKLAH45ONANQ2QV7QN"),
    
    // FxDAO
    ("FxDAO", "CCUN4RXU5VNDHSF4S4RKV4ZJYMX2YWKOH6L4AKEKVNVDQ7HY5QIAO4UB"),
    ("FxDAO", "CDCART6WRSM2K4CKOAOB5YKUVBSJ6KLOVS7ZEJHA4OAQ2FXX7JOHLXIP"),
    
    // Blend
    ("Blend", "CDVQVKOY2YSXS2IC7KN6MNASSHPAO7UN2UR2ON4OI2SKMFJNVAMDX6DP"),
    ("Blend", "CBP7NO6F7FRDHSOFQBT2L2UWYIZ2PU76JKVRYAQTG3KZSQLYAOKIF2WB"),
];

#[no_mangle]
pub extern "C" fn get_positions() {
    let env = EnvClient::empty();
    let body: Request = env.read_request_body();

    let address_id = match stellar_strkey::Strkey::from_string(&body.address).unwrap() {
        stellar_strkey::Strkey::Contract(contract) => contract.0,
        stellar_strkey::Strkey::PublicKeyEd25519(publkey) => publkey.0,
        _ => panic!() // or return error response
    };

    let mut additional_protocols = if let Some(additional) = body.additional {
        additional
    } else {
        vec![]
    };

    let mut all_protocols: Vec<(String, String)> = DEFAULT_SCAN.to_vec().iter().map(|(x, y)| (x.to_string(), y.to_string())).collect();
    all_protocols.append(&mut additional_protocols);
    
    let mut found: Vec<Found> = Vec::new();

    for contract in DEFAULT_SCAN {
        let id = stellar_strkey::Contract::from_string(&contract.1).unwrap().0;
        
        let entries = env.read_contract_entries(id).unwrap();
        for entry in entries {
            if find_address_in_scval(&entry.key, address_id) {
                let zephyr_sdk::soroban_sdk::xdr::LedgerEntryData::ContractData(data) = entry.entry.data else { panic!() };
                let found_entry = Found {
                    protocol: contract.0.to_string(),
                    key: entry.key,
                    value: data.val
                };
                found.push(found_entry)
            }
        }
    }
    
    env.conclude(&found)
}
