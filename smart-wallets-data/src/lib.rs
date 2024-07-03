use serde::{Deserialize, Serialize};
use zephyr_sdk::{prelude::*, soroban_sdk::{Address, Bytes, BytesN, String as SorobanString, Symbol}, DatabaseDerive, EnvClient};

#[derive(DatabaseDerive, Clone, Serialize)]
#[with_name("signers")]
pub struct Signers {
    address: String,
    id: Vec<u8>,
    pubkey: Vec<u8>,
    active: i32
}

fn bytes_to_vec(bytes: Bytes) -> Vec<u8> {
    let mut result = Vec::new();
    
    for byte in bytes.iter() {
        result.push(byte);
    }

    result
}

fn bytesn_to_vec(bytes: BytesN<65>) -> Vec<u8> {
    let mut result = Vec::new();
    
    for byte in bytes.iter() {
        result.push(byte);
    }

    result
}


#[no_mangle]
pub extern "C" fn on_close() {
    let env = EnvClient::new();
    env.log().debug("Converting from str", None);
    let factory_address = SorobanString::from_str(&env.soroban(), "CA4JRRQ52GDJGWIWE7W6J4AUDGLYSEEUUYM4OXERVQ7AUFGS72YNIF65");
    env.log().debug("Done", None);

    for event in env.reader().pretty().soroban_events() {
        if let Some(factory) = event.topics.get(0) {
            let factory = env.try_from_scval::<Address>(factory);
            if let Ok(factory) = factory {
                
                if let Some(topic1) = event.topics.get(1) {
                    let event_type = env.try_from_scval::<Symbol>(&topic1);
                    
                    if let Ok(etype) = event_type {
                        if factory.to_string() == factory_address {
                            env.log().debug("Found factory", None);
                            
                            if etype == Symbol::new(env.soroban(), "add_sig") {
                                let id: Bytes = env.from_scval(&event.topics[2]);
                                let pk: BytesN<65> = env.from_scval(&event.data);

                                env.log().debug("creating signer", None);
                                let signer = Signers {
                                    address: stellar_strkey::Contract(event.contract).to_string(),
                                    id: bytes_to_vec(id),
                                    pubkey: bytesn_to_vec(pk),
                                    active: 0
                                };
                                env.log().debug("created signer", None);

                                env.put(&signer);
                            } if etype == Symbol::new(env.soroban(), "rm_sig") {
                                let id: Bytes = env.from_scval(&event.topics[2]);
                                let id = bytes_to_vec(id);
                                let older: Vec<Signers> = env.read_filter().column_equal_to("id", id.clone()).column_equal_to("active", 0).read().unwrap();
                                let mut older = older[0].clone();
                                older.active = 1;

                                env.update().column_equal_to("id", id).execute(&older).unwrap();
                            }
                        }
                    }
                }
            }
        }
    }
}            

#[derive(Deserialize)]
pub struct SignersByAddressRequest {
    address: String
}

#[derive(Deserialize)]
pub struct AddressBySignerRequest {
    id: Vec<u8>
}

#[no_mangle]
pub extern "C" fn get_signers_by_address() {
    let env = EnvClient::empty();
    let request: SignersByAddressRequest = env.read_request_body();
    let signers: Vec<Signers> = env.read_filter().column_equal_to("address", request.address).column_equal_to("active", 0).read().unwrap();

    env.conclude(&signers)
}

#[no_mangle]
pub extern "C" fn get_address_by_signer() {
    let env = EnvClient::empty();
    let request: AddressBySignerRequest = env.read_request_body();
    let signers: Vec<Signers> = env.read_filter().column_equal_to("id", request.id).column_equal_to("active", 0).read().unwrap();

    env.conclude(&signers)
}