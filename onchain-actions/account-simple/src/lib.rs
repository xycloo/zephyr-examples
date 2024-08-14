#![no_std]

use soroban_sdk::{
    auth::{Context, CustomAccountInterface},
    contract, contracterror, contractimpl, contracttype,
    crypto::Hash,
    Address, BytesN, Env, Symbol, Vec,
};

#[contract]
struct AccountContract;

#[contracttype]
#[derive(Clone)]
pub struct Signature {
    pub public_key: BytesN<32>,
    pub signature: BytesN<64>,
}

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Signer(BytesN<32>),
    BlendPool,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum AccError {
    NotEnoughSigners = 1,
    NegativeAmount = 2,
    BadSignatureOrder = 3,
    UnknownSigner = 4,
    InvalidContext = 5,
}

#[contractimpl]
impl AccountContract {
    // Add other init params here.
    pub fn init(env: Env, signer: BytesN<32>, blend_pool: Address) {
        env.storage().instance().set(&DataKey::Signer(signer), &());
        env.storage()
            .instance()
            .set(&DataKey::BlendPool, &blend_pool);
    }
}

#[contractimpl]
impl CustomAccountInterface for AccountContract {
    type Signature = Signature;
    type Error = AccError;

    #[allow(non_snake_case)]
    fn __check_auth(
        env: Env,
        signature_payload: Hash<32>,
        signature: Signature,
        auth_contexts: Vec<Context>,
    ) -> Result<(), AccError> {
        authenticate(&env, &signature_payload, &signature)?;
        //Ok(())

        // Note that this is actually unsafe and should generally not be used
        // in production. A valid signer could include the Blend submit operation
        // as part of the call stack but perform other malicious operations too.
        let mut result = Err(AccError::InvalidContext);
        
        for context in auth_contexts.iter() {
            match context {
                Context::Contract(c) => {
                    if c.fn_name == Symbol::new(&env, "submit")
                        && c.contract == env.storage().instance().get(&DataKey::BlendPool).unwrap()
                    {
                        result = Ok(());
                    }
                }
                _ => {}
            };
        };

        result
    }
}

fn authenticate(
    env: &Env,
    signature_payload: &Hash<32>,
    signature: &Signature,
) -> Result<(), AccError> {
    if !env
        .storage()
        .instance()
        .has(&DataKey::Signer(signature.public_key.clone())) {
            return Err(AccError::UnknownSigner)
        }

    env.crypto().ed25519_verify(
        &signature.public_key,
        &signature_payload.clone().into(),
        &signature.signature,
    );

    Ok(())
}

mod test;
