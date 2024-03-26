use rs_zephyr_sdk::{bincode, log, stellar_xdr::next::{Limits, WriteXdr}, Condition, DatabaseDerive, DatabaseInteract, EnvClient, ZephyrVal};

#[derive(DatabaseDerive, Clone)]
#[with_name("curr_seq")]
struct Sequence {
    pub current: u32,
}


#[no_mangle]
pub extern "C" fn on_close() {
    let env = EnvClient::new();
    let reader = env.reader();

    let sequence = Sequence {
        current: reader.ledger_sequence()
    };

    if let Some(last) = Sequence::read_to_rows(&env).iter().find(|x| x.current == sequence.current - 1) {
        sequence.update(&env, &[Condition::ColumnEqualTo("current".into(), bincode::serialize(&ZephyrVal::U32(last.current)).unwrap())]);
    } else {
        sequence.put(&env)
    }
}
