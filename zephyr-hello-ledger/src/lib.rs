use zephyr_sdk::{prelude::*, soroban_sdk::xdr::ScVal, Condition, DatabaseDerive, EnvClient};

#[derive(DatabaseDerive, Clone)]
#[with_name("curr_seq")]
struct Sequence {
    pub current: ScVal,
}


#[no_mangle]
pub extern "C" fn on_close() {
    let env = EnvClient::new();
    let reader = env.reader();

    let sequence = Sequence {
        current: ScVal::U32(reader.ledger_sequence())
    };

    if let Some(last) = Sequence::read_to_rows(&env).iter().find(|x| {
        // make sure that our sequence is the latest ledger.
        // this check is enforced only for showing how zephyr doesn't skip
        // a beat and/or to display how to potentially behave for programs that
        // are meant to stop
        let ScVal::U32(num) = x.current else {panic!()};
        let ScVal::U32(current_seq) = sequence.current else {panic!()};
        
        num == current_seq - 1
    }) {
        env.update().column_equal_to_xdr("current", &last.current).execute(&sequence);
    } else {
        sequence.put(&env)
    }
}
