use rs_zephyr_sdk::EnvClient;


#[no_mangle]
pub extern "C" fn on_close() {
    let env = EnvClient::new();
    let reader = env.reader();

    let sequence = reader.ledger_sequence();
    let processing = reader.tx_processing();
    let processing_length = processing.len();

    env.db_write("ledgers", 
    &[
        "sequence", 
        "proc"
    ], 
    &[
        &sequence.to_be_bytes(), 
        &processing_length.to_be_bytes()]
    ).unwrap();
}