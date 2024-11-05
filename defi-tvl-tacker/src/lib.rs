use std::collections::HashMap;

use serde::Deserialize;
use zephyr_sdk::{
    charting::{Dashboard, DashboardBuilder},
    prelude::*,
    soroban_sdk::{
        xdr::{
            AccountId, ContractExecutable, LedgerEntryData, LedgerFootprint, LedgerKey,
            LedgerKeyContractCode, LedgerKeyContractData, MuxedAccount, Operation, OperationBody,
            PublicKey, RestoreFootprintOp, ScAddress, ScSymbol, ScVal, ScVec, SequenceNumber,
            Transaction, TransactionEnvelope, TransactionExt, TransactionV1Envelope, Uint256,
        },
        Address, Symbol,
    },
    utils::{address_to_alloc_string, sign_transaction},
    DatabaseDerive, EnvClient,
};

#[derive(DatabaseDerive, Clone)]
#[with_name("defiopids")]
pub struct Ids {
    current: u32,
    contract: String,
    asset: String,
}

#[derive(DatabaseDerive, Clone)]
#[with_name("sacdefiop")]
pub struct SACDefiOperation {
    sac: String,
    t_count: u32,
    t_volume: i128,
    t_tvl: i128,
    target: String,
    amount: i128,
    operation: String,
    tx: String,
    ledger: u32,
    timestamp: u64,
    id: u32,
}

fn get_current_state_for_target(
    env: &EnvClient,
    asset: String,
    target: String,
) -> (u32, u32, i128, i128) {
    let ids_iter = env
        .read_filter()
        .column_equal_to("contract", target.clone())
        .column_equal_to("asset", asset.clone())
        .read()
        .unwrap();
    let (first_time, id): (bool, Ids) = if let Some(id) = ids_iter.get(0).cloned() {
        (false, id)
    } else {
        (
            true,
            Ids {
                current: 0,
                contract: target.clone(),
                asset: asset.clone(),
            },
        )
    };

    if first_time {
        env.put(&Ids {
            current: id.current + 1,
            contract: id.contract.clone(),
            asset: id.asset.clone(),
        });
    } else {
        env.update()
            .column_equal_to("contract", target.clone())
            .column_equal_to("asset", asset.clone())
            .execute(&Ids {
                current: id.current + 1,
                contract: target.clone(),
                asset: asset.clone(),
            })
            .unwrap();
    }

    let previous_iter = env
        .read_filter()
        .column_equal_to("target", target)
        .column_equal_to("sac", asset)
        .column_equal_to("id", id.current)
        .read()
        .unwrap();
    let previous: Option<&SACDefiOperation> = previous_iter.last();

    if let Some(previous) = previous {
        (
            id.current,
            previous.t_count,
            previous.t_tvl,
            previous.t_volume,
        )
    } else {
        (0, 0, 0, 0)
    }
}

#[no_mangle]
pub extern "C" fn on_close() {
    let env = EnvClient::new();

    env.log().debug(format!("Starting processing"), None);

    for (event, hash) in env.reader().pretty().soroban_events_and_txhash() {
        let contract = event.contract;

        let topics = event.topics.to_vec();
        if let Ok(topic1) = env.try_from_scval::<Symbol>(&topics[0]) {
            if topic1 == Symbol::new(&env.soroban(), "transfer") {
                let sac_string = stellar_strkey::Contract(contract).to_string();
                env.log().debug(format!("got transfer"), None);

                let from = env.try_from_scval::<Address>(&topics[1]);
                let to = env.try_from_scval::<Address>(&topics[2]);
                let amount = env.try_from_scval(&event.data);

                if let (Ok(from), Ok(to), Ok(amount)) = (from, to, amount) {
                    let from = address_to_alloc_string(&env, from);
                    let to = address_to_alloc_string(&env, to);

                    let (from_current_id, from_count, from_tvl, from_volume) =
                        get_current_state_for_target(&env, sac_string.clone(), from.clone());

                    let from_as_target = SACDefiOperation {
                        sac: sac_string.clone(),
                        t_count: from_count + 1,
                        t_tvl: from_tvl - amount,
                        t_volume: from_volume + amount,
                        target: from,
                        amount,
                        operation: "transfer".into(),
                        tx: hex::encode(hash),
                        ledger: env.reader().ledger_sequence(),
                        timestamp: env.reader().ledger_timestamp(),
                        id: from_current_id + 1,
                    };

                    let (to_current_id, to_count, to_tvl, to_volume) =
                        get_current_state_for_target(&env, sac_string.clone(), to.clone());

                    let to_as_target = SACDefiOperation {
                        sac: sac_string,
                        t_count: to_count + 1,
                        t_tvl: to_tvl + amount,
                        t_volume: to_volume + amount,
                        target: to,
                        amount,
                        operation: "transfer".into(),
                        tx: hex::encode(hash),
                        ledger: env.reader().ledger_sequence(),
                        timestamp: env.reader().ledger_timestamp(),
                        id: to_current_id + 1,
                    };

                    env.put(&from_as_target);
                    env.put(&to_as_target);

                    env.log().debug(format!("execution succeded"), None);
                }
            }
        }
    }
}

#[derive(DatabaseDerive, Clone)]
#[with_name("sacdefiop")]
pub struct SACDefiOperationMinify {
    t_volume: i128,
    t_tvl: i128,
    ledger: u32,
    timestamp: u64,
}

#[no_mangle]
pub extern "C" fn dashboard() {
    let env = EnvClient::empty();
    let dasboard = {
        env.log().debug("building chart", None);
        let dashboard = build_sac_dashboard(&env);
        env.log().debug("chart built", None);
        dashboard
    };

    env.conclude(&dasboard)
}

#[derive(Deserialize)]
pub struct Request {
    asset: String,
    searched_sac: String,
    top_targets: Vec<String>,
}

pub fn build_sac_dashboard(env: &EnvClient) -> Dashboard {
    let Request {
        asset,
        searched_sac,
        top_targets,
    } = env.read_request_body();

    let mut dashboard = DashboardBuilder::new(
        &format!("SAC DeFi Activity Dashboard for Asset {asset}",),
        "Explore the DeFi activity of Stellar Asset Contracts",
    );

    for target in top_targets {
        // Get TVL evolution for this target
        let mut tvl_data: Vec<(u32, i128)> = env
            .read_filter()
            .column_equal_to("sac", searched_sac.clone())
            .column_equal_to("target", target.clone())
            .read()
            .unwrap()
            .iter()
            .map(|op: &SACDefiOperationMinify| (op.ledger, op.t_tvl))
            .collect();
        tvl_data.sort_by_key(|&(ledger, _)| ledger);

        // Add TVL evolution chart for this target
        dashboard = dashboard.add_line_chart(
            &format!("TVL Evolution for {} - Target: {}", searched_sac, target),
            tvl_data
                .iter()
                .map(|(ledger, _)| ledger.to_string())
                .collect(),
            vec![(
                &format!("TVL of {}", target),
                tvl_data
                    .iter()
                    .map(|(_, tvl)| ((*tvl as f64) / 10_000_000.0) as i64)
                    .collect(),
            )],
        );

        // Get volume data for this target
        let mut volume_data: Vec<(u32, i128)> = env
            .read_filter()
            .column_equal_to("sac", searched_sac.clone())
            .column_equal_to("target", target.clone())
            .read()
            .unwrap()
            .iter()
            .map(|op: &SACDefiOperationMinify| (op.ledger, op.t_volume))
            .collect();
        volume_data.sort_by_key(|&(ledger, _)| ledger);

        // Add volume evolution chart for this target
        dashboard = dashboard.add_line_chart(
            &format!("Volume Evolution for {} - Target: {}", searched_sac, target),
            volume_data
                .iter()
                .map(|(ledger, _)| ledger.to_string())
                .collect(),
            vec![(
                &format!("Volume of {} for {}", searched_sac, target),
                volume_data
                    .iter()
                    .map(|(_, volume)| ((*volume as f64) / 10_000_000.0) as i64)
                    .collect(),
            )],
        );
    }

    dashboard.build()
}
