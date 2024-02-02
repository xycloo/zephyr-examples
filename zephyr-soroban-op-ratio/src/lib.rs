use rs_zephyr_sdk::{
    stellar_xdr::next::{
        FeeBumpTransactionInnerTx, Operation, OperationBody, TransactionEnvelope, VecM,
    },
    EnvClient,
};

#[no_mangle]
pub extern "C" fn on_close() {
    let env = EnvClient::new();

    let (soroban, ratio) = {
        let mut strict_soroban = 0;
        let mut classic = 0;

        for envelope in env.reader().envelopes() {
            match envelope {
                TransactionEnvelope::Tx(v1) => {
                    count_ops(&v1.tx.operations, &mut strict_soroban, &mut classic)
                }
                TransactionEnvelope::TxFeeBump(feebump) => {
                    let FeeBumpTransactionInnerTx::Tx(v1) = feebump.tx.inner_tx;
                    count_ops(&v1.tx.operations, &mut strict_soroban, &mut classic);
                }
                TransactionEnvelope::TxV0(v0) => {
                    count_ops(&v0.tx.operations, &mut strict_soroban, &mut classic)
                }
            }
        }

        (
            strict_soroban,
            strict_soroban as f64 / (strict_soroban as f64 + classic as f64),
        )
    };

    if soroban > 0 {
        env.db_write(
            "opratio",
            &["sequence", "soroban", "ratio"],
            &[
                &env.reader().ledger_sequence().to_be_bytes(),
                &soroban.to_be_bytes(),
                &ratio.to_be_bytes(),
            ],
        )
        .unwrap();
    }
}

fn count_ops(ops: &VecM<Operation, 100>, strict_soroban: &mut i32, classic: &mut i32) {
    for op in ops.iter() {
        match op.body {
            OperationBody::InvokeHostFunction(_) => *strict_soroban += 1,
            OperationBody::ExtendFootprintTtl(_) => *strict_soroban += 1,
            OperationBody::RestoreFootprint(_) => *strict_soroban += 1,
            _ => *classic += 1,
        }
    }
}
