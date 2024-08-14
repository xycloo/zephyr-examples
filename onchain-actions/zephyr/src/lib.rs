use urlencoding::encode;
use zephyr_sdk::{
    prelude::*, soroban_sdk::{
        self, map, vec,
        xdr::{self, Limits, Transaction, TransactionEnvelope, TransactionExt, TransactionV1Envelope, WriteXdr},
        Address, BytesN, IntoVal, Map, Symbol, Val, 
    }, utils::{
        add_contract_to_footprint, address_from_str, build_authorization_preimage, ed25519_sign,
        sha256, sign_transaction, soroban_string_to_alloc_string
    }, AgnosticRequest, EnvClient, PrettyContractEvent
};

// Note: these small fee adjustements are needed because:
// 1. The ZVM is using a newer version of the RPC.
// 2. We are only simulating once, thus need to account for
// signature verification too.
const INSTRUCTIONS_FIX: u32 = 2000292;
const WRITE_BYTES_FIX: u32 = 200;
const READ_BYTES_FIX: u32 = 100000;
const RESOURCE_FEE_FIX: i64 = 9000965;
const FEE_FIX: u32 = 9950000;

// Signature's ledgers to live.
const SIGNATURE_DURATION: u32 = 100;
const NETWORK: &'static str = "Test SDF Network ; September 2015";

const CONTRACT: &'static str = "CDWCIQMI3EHKGK32SVICLUBFVBT5VKNG72O3G7KNJOLSIVA4WDVQ2IYX";

const SOURCE_ACCOUNT: &'static str = "GDQ47JRRX2SQ7YHM6FAMMDC4K5EXFZOYPWZCFAILKEE3IYTZAQEFGR3M";
const SECRET: &'static str = "SDD7ZVTGRP2A5PX3G6FZ56HVCFPBWPGM3XMYYE5EEOEV5FM5KIJGGWOS";

const ACCOUNT: &'static str = "CC7KO34Z7YWG44B7HAHBJMIBJWXGDSXEYJB43LG7NJSNDLX3G3D3XUYY";
const ACCOUNT_HASH: &'static str =
    "e530fc0b88328d7116801643e4b5decbdf94c3e5114e1459cbed45d995d62e08";

#[no_mangle]
pub extern "C" fn on_close() {
    let env = EnvClient::new();
    let ybx_contract = stellar_strkey::Contract::from_string(&CONTRACT).unwrap().0;
    let searched_events: Vec<PrettyContractEvent> = {
        let events = env.reader().pretty().soroban_events();
        events
            .iter()
            .filter_map(|x| {
                if x.contract == ybx_contract {
                    Some(x.clone())
                } else {
                    None
                }
            })
            .collect()
    };

    for event in searched_events {
        let action: Symbol = env.from_scval(&event.topics[0]);
        let token: Address = env.from_scval(&event.topics[1]);

        if action == Symbol::new(env.soroban(), "borrow")
            && &soroban_string_to_alloc_string(&env, token.to_string())
                == "CAQCFVLOBK5GIULPNZRGATJJMIZL5BSP7X5YJVMGCPTUEPFM4AVSRCJU"
        {
            execute_transaction(&env);
        }
    }
}

fn execute_transaction(env: &EnvClient) {
    let account = stellar_strkey::ed25519::PublicKey::from_string(&SOURCE_ACCOUNT)
        .unwrap()
        .0;
    let contract = stellar_strkey::Contract::from_string(&CONTRACT).unwrap().0;

    let sequence = env
        .read_account_from_ledger(account)
        .unwrap()
        .unwrap()
        .seq_num;

        env.log().debug("Got sequence", None);

    let map: Map<Symbol, Val> = map![
        &env.soroban(),
        (
            Symbol::new(&env.soroban(), "request_type"),
            2_u32.into_val(env.soroban()),
        ),
        (
            Symbol::new(&env.soroban(), "address"),
            Address::from_string(&zephyr_sdk::soroban_sdk::String::from_str(
                &env.soroban(),
                "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC",
            ))
            .into_val(env.soroban()),
        ),
        (
            Symbol::new(&env.soroban(), "amount"),
            100_000_000_i128.into_val(env.soroban()),
        )
    ];
    
    let args: soroban_sdk::Vec<Val> = vec![
        &env.soroban(),
        address_from_str(env, ACCOUNT).into_val(env.soroban()),
        address_from_str(env, ACCOUNT).into_val(env.soroban()),
        address_from_str(env, ACCOUNT).into_val(env.soroban()),
        vec![&env.soroban(), map].into_val(env.soroban()),
    ];

    let tx = env.simulate_contract_call_to_tx(
        SOURCE_ACCOUNT.into(),
        sequence as i64 + 1,
        contract,
        Symbol::new(&env.soroban(), "submit"),
        args,
    );
    
    if tx.clone().unwrap().error.is_some() {
        env.log().debug(format!("{:?}", tx.clone().unwrap().error), None);
    } else {

        let mut tx_with_signed_auth = sign_auth_entries(
            env,
            TransactionEnvelope::from_xdr_base64(tx.unwrap().tx.unwrap(), Limits::none()).unwrap(),
        );

        env.log().debug("signed", None);

        let TransactionExt::V1(mut v1ext) = tx_with_signed_auth.ext else {
            panic!()
        };
        let mut r = v1ext.resources;

        // Adding the contract code and instance to the footprint.
        // NB: this is needed since simulation doesn't currently account for the
        // contracts in the auth stack that aren't directly invoked (such as our custom account).
        let mut footprint = r.footprint;
        add_contract_to_footprint(
            &mut footprint,
            &ACCOUNT,
            &hex::decode(ACCOUNT_HASH).unwrap(),
        );
        r.footprint = footprint;

        // Note that currently zephyr is operating on a newer simulation branch, so we need to slightly adjust
        // simulation resource parameters.
        r.instructions += INSTRUCTIONS_FIX;
        r.write_bytes += WRITE_BYTES_FIX;
        r.read_bytes += READ_BYTES_FIX;
        v1ext.resource_fee += RESOURCE_FEE_FIX;
        v1ext.resources = r;
        tx_with_signed_auth.ext = TransactionExt::V1(v1ext);
        tx_with_signed_auth.fee += FEE_FIX;

        let signed = sign_transaction(tx_with_signed_auth, &NETWORK, &SECRET);
        env.send_web_request(AgnosticRequest {
            body: Some(format!("tx={}", encode(&signed))),
            url: "https://horizon-testnet.stellar.org/transactions".to_string(),
            method: zephyr_sdk::Method::Post,
            headers: std::vec![(
                "Content-Type".to_string(),
                "application/x-www-form-urlencoded".to_string()
            )],
        });

        env.log().debug(signed, None);
    }
}

fn sign_auth_entries(env: &EnvClient, tx: TransactionEnvelope) -> Transaction {
    let new_sequence = env.reader().ledger_sequence() + SIGNATURE_DURATION;
    let TransactionEnvelope::Tx(TransactionV1Envelope { mut tx, .. }) = tx else {
        panic!()
    };
    let source = tx.operations.to_vec()[0].source_account.clone();
    let xdr::OperationBody::InvokeHostFunction(mut host_function) =
        tx.operations.to_vec()[0].body.clone()
    else {
        panic!()
    };
    let mut auth = host_function.auth.to_vec()[0].clone();
    let xdr::SorobanCredentials::Address(mut credentials) = auth.clone().credentials else {
        panic!()
    };

    let preimage = build_authorization_preimage(
        credentials.nonce,
        new_sequence,
        auth.clone().root_invocation,
    );
    let payload = sha256(&preimage.to_xdr(Limits::none()).unwrap());
    let (public, signature) = ed25519_sign(&SECRET, &payload);
    let public = public.to_bytes();

    let signature: Map<Val, Val> = map![
        &env.soroban(),
        (
            Symbol::new(&env.soroban(), "signature").into_val(env.soroban()),
            BytesN::from_array(&env.soroban(), &signature).into_val(env.soroban())
        ),
        (
            Symbol::new(&env.soroban(), "public_key").into_val(env.soroban()),
            BytesN::from_array(&env.soroban(), &public).into_val(env.soroban())
        ),
    ];

    credentials.signature_expiration_ledger = new_sequence;
    credentials.signature = env.to_scval(signature);
    auth.credentials = xdr::SorobanCredentials::Address(credentials);
    host_function.auth = std::vec![auth].try_into().unwrap();

    tx.operations = std::vec![xdr::Operation {
        source_account: source,
        body: xdr::OperationBody::InvokeHostFunction(host_function)
    }]
    .try_into()
    .unwrap();

    tx
}

#[cfg(test)]
mod test {
    use ledger_meta_factory::TransitionPretty;
    use stellar_xdr::next::{Hash, Int128Parts, ScSymbol, ScVal};
    use zephyr_sdk::testutils::TestHost;

    fn add_borrow(transition: &mut TransitionPretty) {
        transition.inner.set_sequence(2000);
        transition
            .contract_event(
                "CDWCIQMI3EHKGK32SVICLUBFVBT5VKNG72O3G7KNJOLSIVA4WDVQ2IYX",
                vec![
                    ScVal::Symbol(ScSymbol("borrow".try_into().unwrap())),
                    ScVal::Address(stellar_xdr::next::ScAddress::Contract(Hash(
                        stellar_strkey::Contract::from_string(
                            "CAQCFVLOBK5GIULPNZRGATJJMIZL5BSP7X5YJVMGCPTUEPFM4AVSRCJU",
                        )
                        .unwrap()
                        .0,
                    ))),
                ],
                ScVal::I128(Int128Parts {
                    hi: 0,
                    lo: 100000000,
                }),
            )
            .unwrap();
    }

    #[tokio::test]
    async fn deposit() {
        let env = TestHost::default();
        let mut program = env.new_program("./target/wasm32-unknown-unknown/release/zephyr.wasm");
        let mut empty = TransitionPretty::new();
        program.set_transition(empty.inner.clone());
        add_borrow(&mut empty);
        program.set_transition(empty.inner.clone());
        let invocation = program.invoke_vm("on_close").await;
        assert!(invocation.is_ok());
        let inner_invocation = invocation.unwrap();
        assert!(inner_invocation.is_ok());
    }
}
