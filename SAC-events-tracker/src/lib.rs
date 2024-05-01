//! NOTE:
//! This example is outdated and should not be used as reference.
//! 
//! 
use zephyr_sdk::{
    soroban_sdk::xdr::{ContractEventBody, Limits, TransactionMeta, WriteXdr},
    EnvClient,
};

#[no_mangle]
pub extern "C" fn on_close() {
    let env = EnvClient::new();
    let reader = env.reader();

    let sequence = reader.ledger_sequence();
    let processing = reader.tx_processing();

    let sacs = env.db_read("sacs", &["contract"]).unwrap();
    let tracked_deployed_sacs: Vec<&Vec<u8>> = sacs.rows.iter().map(|row| &row.row[0].0).collect();

    for tx_processing in processing {
        if let TransactionMeta::V3(meta) = &tx_processing.tx_apply_processing {
            if let Some(soroban) = &meta.soroban_meta {
                if !soroban.events.is_empty() {
                    for event in soroban.events.iter() {
                        let contract_id = event.contract_id.as_ref().unwrap().0;
                        if tracked_deployed_sacs.contains(&contract_id.to_vec().as_ref()) {
                            let (topics, data) = match &event.body {
                                ContractEventBody::V0(v0) => (
                                    v0.topics
                                        .iter()
                                        .map(|topic| topic.to_xdr(Limits::none()).unwrap())
                                        .collect::<Vec<Vec<u8>>>(),
                                    v0.data.to_xdr(Limits::none()).unwrap(),
                                ),
                            };
                            env.db_write(
                                "sac_event",
                                &[
                                    "sequence", "contract", "topic1", "topic2", "topic3", "topic4",
                                    "data",
                                ],
                                &[
                                    &sequence.to_be_bytes(),
                                    &contract_id,
                                    &topics.get(0).unwrap_or(&vec![]),
                                    &topics.get(1).unwrap_or(&vec![]),
                                    &topics.get(2).unwrap_or(&vec![]),
                                    &topics.get(3).unwrap_or(&vec![]),
                                    &data,
                                ],
                            )
                            .unwrap()
                        }
                    }
                }
            }
        }
    }
}
