use rs_zephyr_sdk::{
    stellar_xdr::next::{
        ContractIdPreimage, FeeBumpTransactionInnerTx, HostFunction, LedgerKey, Limits,
        OperationBody, ScAddress, TransactionEnvelope, TransactionExt, TransactionResultMeta,
        TransactionResultResult, TransactionV1Envelope, WriteXdr,
    },
    EnvClient,
};

struct CreatedSAC {
    contract_id: [u8; 32],
    asset: Vec<u8>,
}

#[no_mangle]
pub extern "C" fn on_close() {
    let env = EnvClient::new();
    let reader = env.reader();

    let envelopes = reader.envelopes();
    let processing = reader.tx_processing();
    let current_ledger = reader.ledger_sequence();

    let mut created = Vec::new();

    for (idx, envelope) in envelopes.iter().enumerate() {
        match envelope {
            TransactionEnvelope::Tx(tx) => write_from_v1(idx, tx, &processing, &mut created),

            // v0 txs cannot inlcude soroban data
            TransactionEnvelope::TxV0(_) => (),

            TransactionEnvelope::TxFeeBump(tx) => match &tx.tx.inner_tx {
                FeeBumpTransactionInnerTx::Tx(tx) => {
                    write_from_v1(idx, &tx, &processing, &mut created)
                }
            },
        }
    }

    // add all created SACs to the database.
    for sac in &created {
        env.db_write(
            "sacs",
            &["contract", "asset"],
            &[&sac.contract_id, &sac.asset],
        )
        .unwrap();
    }

}

fn write_from_v1(
    idx: usize,
    tx: &TransactionV1Envelope,
    processing: &Vec<TransactionResultMeta>,
    created: &mut Vec<CreatedSAC>,
) {
    match &tx.tx.operations.get(0).unwrap().body {
        // we search for create SAC operations
        OperationBody::InvokeHostFunction(op) => {
            if let HostFunction::CreateContract(create_contract) = &op.host_function {
                if let ContractIdPreimage::Asset(asset) = &create_contract.contract_id_preimage {
                    let matching_processing = processing.get(idx).unwrap();

                    // we make sure that the tx was successful
                    if let TransactionResultResult::TxSuccess(_) =
                        matching_processing.result.result.result
                    {
                        if let TransactionExt::V1(soroban) = &tx.tx.ext {
                            if let LedgerKey::ContractData(data) =
                                &soroban.resources.footprint.read_write[0]
                            {
                                if let ScAddress::Contract(contract) = &data.contract {
                                    created.push(CreatedSAC {
                                        contract_id: contract.0,
                                        asset: asset.to_xdr(Limits::none()).unwrap(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        _ => (),
    }
}
