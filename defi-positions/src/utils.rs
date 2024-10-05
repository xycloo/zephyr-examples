use zephyr_sdk::soroban_sdk::xdr::{PublicKey, ScAddress, ScVal};


pub fn find_address_in_scval(val: &ScVal, address: [u8; 32]) -> bool {
    match val {
        ScVal::Address(object) => match object {
            ScAddress::Account(pubkey) => {
                if let PublicKey::PublicKeyTypeEd25519(pubkey) = &pubkey.0 {
                    return pubkey.0 == address;
                }
            }
            ScAddress::Contract(hash) => {
                return hash.0 == address;
            }
        },
        ScVal::Vec(Some(scvec)) => {
            for val in scvec.0.to_vec() {
                if find_address_in_scval(&val, address) {
                    return true;
                }
            }
        }
        ScVal::Map(Some(scmap)) => {
            for kv in scmap.0.to_vec() {
                if find_address_in_scval(&kv.key, address)
                    || find_address_in_scval(&kv.val, address)
                {
                    return true;
                }
            }
        }
        _ => {}
    }

    false
}