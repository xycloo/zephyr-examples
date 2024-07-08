use zephyr_sdk::{
    prelude::*,
    soroban_sdk::{
        self, symbol_short,
        xdr::{ScString, ScVal},
        Address,
    },
    DatabaseDerive, EnvClient, PrettyContractEvent,
};

pub const CONTRACT_ADDRESS: &str = "CCKTUGINX4LEGTAFWE2U6BK7C4NSKXNWTYXJ5FSGBUQXJUL7ZHQZWLCC";

#[derive(DatabaseDerive, Clone, Default)]
#[with_name("deposited")]
pub struct Deposit {
    id: i32,
    from_addr: String,
    amount: i64,
    total: i64,
}

#[derive(DatabaseDerive, Clone, Default)]
#[with_name("id")]
pub struct Id {
    deposit: i32,
}

impl Id {
    fn get(env: &EnvClient) -> Option<i32> {
        let ids: Vec<Id> = env.read();
        ids.get(0).map(|x| x.deposit)
    }

    fn increment(env: &EnvClient) {
        if let Some(id) = Self::get(env) {
            let res = env
                .update()
                .column_equal_to("deposit", id)
                .execute(&Self { deposit: id + 1 });
            env.log().debug(format!("{:?}", res), None)
        } else {
            let _ = env.put(&Self { deposit: 0 });
        }
    }
}

impl Deposit {
    fn entry(env: &EnvClient, from: &ScVal, amount: &ScVal, is_deposit: bool) {
        let address: Address = env.from_scval(from);
        let from_addr = {
            let soroban_string = env.to_scval(address.to_string());
            let ScVal::String(ScString(string)) = soroban_string else {
                panic!()
            };
            string.try_into().unwrap()
        };
        let amount: i128 = env.from_scval(amount);
        let current_id = Id::get(env);

        let (id, total) = if let Some(id) = current_id {
            let previous: Vec<Deposit> =
                env.read_filter().column_equal_to("id", id).read().unwrap();
            let previous: Deposit = previous.get(0).map(|x| x.clone()).unwrap_or_default();

            (id + 1, previous.total)
        } else {
            (0, 0)
        };

        let deposit = Self {
            id,
            from_addr,
            amount: if is_deposit {
                amount as i64
            } else {
                -amount as i64
            },
            total: if is_deposit {
                total + amount as i64
            } else {
                total - amount as i64
            },
        };

        env.log().debug("Incrementing", None);
        Id::increment(env);

        env.log().debug("Inserting", None);
        env.put(&deposit);
    }
}

#[no_mangle]
pub extern "C" fn on_close() {
    let env = EnvClient::new();
    let events = env.reader().pretty().soroban_events();
    let searched_events: Vec<&PrettyContractEvent> = events
        .iter()
        .filter(|x| {
            x.contract
                == stellar_strkey::Contract::from_string(CONTRACT_ADDRESS)
                    .unwrap()
                    .0
        })
        .collect();

    for event in searched_events {
        if symbol_short!("deposit") == env.from_scval(&event.topics[0]) {
            Deposit::entry(&env, &event.topics[1], &event.data, true)
        } else if symbol_short!("withdraw") == env.from_scval(&event.topics[0]) {
            env.log().debug("About to withdraw", None);
            Deposit::entry(&env, &event.topics[1], &event.data, false)
        }
    }
}

#[cfg(test)]
mod test {
    use ledger_meta_factory::TransitionPretty;
    use stellar_xdr::next::{Hash, Int128Parts, ScSymbol, ScVal};
    use zephyr_sdk::testutils::TestHost;

    fn add_deposit(transition: &mut TransitionPretty) {
        transition.inner.set_sequence(2000);
        transition
            .contract_event(
                "CCKTUGINX4LEGTAFWE2U6BK7C4NSKXNWTYXJ5FSGBUQXJUL7ZHQZWLCC",
                vec![
                    ScVal::Symbol(ScSymbol("deposit".try_into().unwrap())),
                    ScVal::Address(stellar_xdr::next::ScAddress::Contract(Hash([8; 32]))),
                ],
                ScVal::I128(Int128Parts {
                    hi: 0,
                    lo: 100000000,
                }),
            )
            .unwrap();
    }

    fn add_withdraw(transition: &mut TransitionPretty) {
        transition.inner.set_sequence(2010);
        transition
            .contract_event(
                "CCKTUGINX4LEGTAFWE2U6BK7C4NSKXNWTYXJ5FSGBUQXJUL7ZHQZWLCC",
                vec![
                    ScVal::Symbol(ScSymbol("withdraw".try_into().unwrap())),
                    ScVal::Address(stellar_xdr::next::ScAddress::Contract(Hash([8; 32]))),
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
        let mut program = env.new_program("./target/wasm32-unknown-unknown/release/testing.wasm");

        let mut db = env.database("postgres://postgres:postgres@localhost:5432");
        db.load_table(0, "deposited", vec!["id", "from_addr", "amount", "total"])
            .await;
        db.load_table(0, "id", vec!["deposit"]).await;

        assert_eq!(db.get_rows_number(0, "id").await.unwrap(), 0);
        assert_eq!(db.get_rows_number(0, "deposited").await.unwrap(), 0);

        let mut empty = TransitionPretty::new();
        program.set_transition(empty.inner.clone());

        let invocation = program.invoke_vm("on_close").await;
        assert!(invocation.is_ok());
        let inner_invocation = invocation.unwrap();
        assert!(inner_invocation.is_ok());

        assert_eq!(db.get_rows_number(0, "id").await.unwrap(), 0);
        assert_eq!(db.get_rows_number(0, "deposited").await.unwrap(), 0);

        // After deposit

        add_deposit(&mut empty);
        program.set_transition(empty.inner.clone());

        let invocation = program.invoke_vm("on_close").await;
        assert!(invocation.is_ok());
        let inner_invocation = invocation.unwrap();
        assert!(inner_invocation.is_ok());

        assert_eq!(db.get_rows_number(0, "id").await.unwrap(), 1);
        assert_eq!(db.get_rows_number(0, "deposited").await.unwrap(), 1);

        db.close().await
    }

    #[tokio::test]
    async fn withdraw() {
        let env = TestHost::default();
        let mut program = env.new_program("./target/wasm32-unknown-unknown/release/testing.wasm");

        let mut db = env.database("postgres://postgres:postgres@localhost:5432");
        db.load_table(0, "deposited", vec!["id", "from_addr", "amount", "total"])
            .await;
        db.load_table(0, "id", vec!["deposit"]).await;

        assert_eq!(db.get_rows_number(0, "id").await.unwrap(), 0);
        assert_eq!(db.get_rows_number(0, "deposited").await.unwrap(), 0);

        let mut empty = TransitionPretty::new();
        program.set_transition(empty.inner.clone());

        let invocation = program.invoke_vm("on_close").await;
        assert!(invocation.is_ok());
        let inner_invocation = invocation.unwrap();
        assert!(inner_invocation.is_ok());

        assert_eq!(db.get_rows_number(0, "id").await.unwrap(), 0);
        assert_eq!(db.get_rows_number(0, "deposited").await.unwrap(), 0);

        // After deposit

        add_deposit(&mut empty);
        program.set_transition(empty.inner.clone());

        let invocation = program.invoke_vm("on_close").await;
        assert!(invocation.is_ok());
        let inner_invocation = invocation.unwrap();
        assert!(inner_invocation.is_ok());

        assert_eq!(db.get_rows_number(0, "id").await.unwrap(), 1);
        assert_eq!(db.get_rows_number(0, "deposited").await.unwrap(), 1);

        // After deposit + withdrawal

        add_withdraw(&mut empty);
        program.set_transition(empty.inner);

        let invocation = program.invoke_vm("on_close").await;
        assert!(invocation.is_ok());
        let inner_invocation = invocation.unwrap();
        assert!(inner_invocation.is_ok());

        assert_eq!(db.get_rows_number(0, "id").await.unwrap(), 1);
        assert_eq!(db.get_rows_number(0, "deposited").await.unwrap(), 3);

        db.close().await
    }
}
