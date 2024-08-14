#![cfg(test)]
extern crate std;

use ed25519_dalek::Keypair;
use ed25519_dalek::PublicKey;
use ed25519_dalek::SecretKey;
use ed25519_dalek::Signer;
use rand::thread_rng;
use soroban_sdk::auth::ContractContext;
use soroban_sdk::contract;
use soroban_sdk::contractimpl;
use soroban_sdk::contracttype;
use soroban_sdk::xdr::ToXdr;
use soroban_sdk::String;
use soroban_sdk::Val;
use soroban_sdk::{
    auth::Context, testutils::BytesN as _, vec, Address, BytesN, Env, IntoVal, Symbol,
};

use crate::AccError;
use crate::{Signature, AccountContract, AccountContractClient};

fn generate_keypair() -> Keypair {
    Keypair::generate(&mut thread_rng())
}

fn signer_public_key(e: &Env, signer: &Keypair) -> BytesN<32> {
    signer.public.to_bytes().into_val(e)
}

fn create_account_contract(e: &Env) -> AccountContractClient {
    AccountContractClient::new(e, &e.register_contract(None, AccountContract {}))
}

fn sign(e: &Env, signer: &Keypair, payload: &BytesN<32>) -> Val {
    let d: Val = Signature {
        public_key: signer_public_key(e, signer),
        signature: signer
            .sign(payload.to_array().as_slice())
            .to_bytes()
            .into_val(e),
    }
    .into_val(e);

    ().into_val(e)
}

fn blend_auth_context(e: &Env, blend_id: &Address, fn_name: Symbol) -> Context {
    Context::Contract(ContractContext {
        contract: blend_id.clone(),
        fn_name,
        args: ((), (), ()).into_val(e),
    })
}

#[contract]
pub struct BlendMock;

#[contractimpl]
impl BlendMock {
    pub fn submit(env: Env) {}
}

#[test]
fn get_signer() {
    let secret = SecretKey::from_bytes(
        &stellar_strkey::ed25519::PrivateKey::from_string(
            "SDD7ZVTGRP2A5PX3G6FZ56HVCFPBWPGM3XMYYE5EEOEV5FM5KIJGGWOS",
        )
        .unwrap()
        .0,
    )
    .unwrap();
    let public: PublicKey = (&secret).into();
    std::println!("{:?}", public.as_bytes());
}

#[derive(Clone)]
#[contracttype]
pub struct Request {
    pub request_type: u32,
    pub address: Address, // asset address or liquidatee
    pub amount: i128,
}

#[test]
fn test_blend_auth() {
    let env = Env::default();
    env.mock_all_auths();

    let requests = vec![
        &env,
        Request {
            request_type: 2,
            address: Address::from_string(&String::from_str(
                &env,
                "CAQCFVLOBK5GIULPNZRGATJJMIZL5BSP7X5YJVMGCPTUEPFM4AVSRCJU",
            )),
            amount: 1000_000_000,
        },
    ];

    std::println!("{:?}", requests.to_xdr(&env));

    let blend = env.register_contract(None, BlendMock);

    let account_contract = create_account_contract(&env);

    let signer = generate_keypair();
    account_contract.init(&signer_public_key(&env, &signer), &blend);

    let payload = BytesN::random(&env);

    env.try_invoke_contract_check_auth::<AccError>(
        &account_contract.address,
        &payload,
        sign(&env, &signer, &payload),
        &vec![
            &env,
            blend_auth_context(&env, &blend, Symbol::new(&env, "submit")),
        ],
    )
    .unwrap();
}
