use crate::types::{
    auction_from_u32, AggregatedData, Borrowed, Collateral, PoolDataKey, Positions,
    StellarAssetContractMetadata,
};
use std::{
    collections::{BTreeMap, HashMap},
    ops::AddAssign,
};
use zephyr_sdk::{
    charting::{Dashboard, DashboardBuilder, DashboardEntry, Table},
    soroban_sdk::{
        xdr::{LedgerEntryData, ScString, ScVal},
        Address, String as SorobanString, Symbol,
    },
    utils, ContractDataEntry, EnvClient,
};

pub const STROOP: i128 = 10_000_000;
pub const DAY_TIMEFRAME: i64 = 86_400;
pub const WEEK_TIMEFRAME: i64 = DAY_TIMEFRAME * 7;
pub const MONTH_TIMEFRAME: i64 = DAY_TIMEFRAME * 30;

pub fn soroban_string_to_string(env: &EnvClient, string: SorobanString) -> String {
    let sc_val: ScVal = env.to_scval(string);
    if let ScVal::String(ScString(s)) = sc_val {
        let s = s.to_utf8_string().unwrap();
        s
    } else {
        panic!("value is not a string");
    }
}

pub fn get_from_instance(env: &EnvClient, contract: &str, str_: &str) -> ScVal {
    let instance = env
        .read_contract_instance(stellar_strkey::Contract::from_string(&contract).unwrap().0)
        .unwrap()
        .unwrap();
    let LedgerEntryData::ContractData(data) = instance.entry.data else {
        panic!()
    };
    let ScVal::ContractInstance(instance) = data.val else {
        panic!()
    };
    let val = instance
        .storage
        .as_ref()
        .unwrap()
        .0
        .iter()
        .find(|entry| entry.key == env.to_scval(Symbol::new(&env.soroban(), str_)));

    val.unwrap().val.clone()
}

fn get_all_entries(env: &EnvClient, contract: &str) -> Vec<ContractDataEntry> {
    env.read_contract_entries(stellar_strkey::Contract::from_string(contract).unwrap().0)
        .unwrap()
}

fn get_from_ledger(env: &EnvClient, contract: &str) -> i64 {
    let mut total_positions = 0_i64;
    let entries = get_all_entries(env, contract);

    for entry in entries {
        let LedgerEntryData::ContractData(data) = entry.entry.data else {
            env.log()
                .debug(format!("not contract data {:?}", entry.entry.data), None);
            panic!()
        };
        if let Ok(entry_key) = env.try_from_scval::<PoolDataKey>(&data.key) {
            match entry_key {
                PoolDataKey::Positions(_) => total_positions += 1,
                _ => (),
            }
        }
    }

    total_positions
}

pub fn aggregate_data<'a>(
    timestamp: i64,
    //supplies: &'a Vec<Supply>,
    collaterals: &'a Vec<Collateral>,
    borroweds: &'a Vec<Borrowed>,
) -> HashMap<&'a str, HashMap<&'a str, AggregatedData>> {
    let env = EnvClient::empty();
    let mut aggregated_data: HashMap<&'a str, HashMap<&'a str, AggregatedData>> = HashMap::new();
    //let mut volume_24hrs: Vec<(&'a str, u64)> = Vec::new();
    //let mut volume_week = HashMap::new();

    env.log().debug("hashmaps", None);

    /*for supply in supplies.iter().rev().take(500) {
        let pool = &supply.pool;  // Convert pool to string for hashmap key
        let asset = &supply.asset;  // Convert asset to string for hashmap key
        let supply_value = supply.supply;
        let ledger =supply.ledger;

        aggregated_data
        .entry(&pool)
            .or_insert_with(HashMap::new)
            .entry(&asset)
            .or_insert_with(AggregatedData::new)
            .add_supply(ledger, supply_value);
    }*/

    for collateral in collaterals.iter() {
        let pool = &collateral.pool;
        let asset = &collateral.asset;
        let collateral_value = collateral.clateral;
        let ledger = collateral.ledger;
        let entry_timestamp = collateral.timestamp;

        aggregated_data
            .entry(&pool)
            .or_insert_with(HashMap::new)
            .entry(&asset)
            .or_insert_with(AggregatedData::new)
            .add_collateral(
                ledger,
                collateral_value,
                collateral.delta,
                entry_timestamp,
                timestamp,
            )
    }

    for borrowed in borroweds.iter() {
        let pool = &borrowed.pool;
        let asset = &borrowed.asset;
        let borrowed_value = borrowed.borrowed;
        let ledger = borrowed.ledger;
        let entry_timestamp = borrowed.timestamp;

        aggregated_data
            .entry(&pool)
            .or_insert_with(HashMap::new)
            .entry(&asset)
            .or_insert_with(AggregatedData::new)
            .add_borrowed(
                ledger,
                borrowed_value,
                borrowed.delta,
                entry_timestamp,
                timestamp,
            )
    }

    aggregated_data
}

pub fn build_dashboard<'a>(
    env: &EnvClient,
    aggregated_data: HashMap<&'a str, HashMap<&'a str, AggregatedData>>,
    collaterals: &Vec<Collateral>,
    borroweds: &Vec<Borrowed>,
) -> Dashboard {
    let mut dashboard = DashboardBuilder::new(
        "Blend Porotocol Dashboard",
        "Explore the Blend protocol's mainnet activity",
    );

    for (pool, assets) in aggregated_data {
        let positions_count = get_from_ledger(env, &pool);
        dashboard = dashboard.add_table(
            "Current Unique Users With Positions",
            vec!["count".into()],
            vec![vec![positions_count.to_string()]],
        );

        let val = get_from_instance(env, pool, "Name");
        let ScVal::String(string) = val else { panic!() };
        let pool = string.to_utf8_string().unwrap();

        for (asset, data) in assets {
            let meta: StellarAssetContractMetadata =
                env.from_scval(&get_from_instance(env, asset, "METADATA"));
            let denom = soroban_string_to_string(env, meta.symbol);
            let asset = denom.clone();

            {
                let categories: Vec<&str> = vec!["Supply", "Collateral", "Borrowed"];

                dashboard = dashboard.add_bar_chart(
                    "All-time distribution",
                    &format!("Pool: {}, Asset {}", pool, asset),
                    categories,
                    vec![
                        data.total_supply as i64 / STROOP as i64,
                        data.total_collateral as i64 / STROOP as i64,
                        data.total_borrowed as i64 / STROOP as i64,
                    ],
                );
            };

            {
                let line_data: Vec<i64> = data
                    .collateral
                    .iter()
                    .map(|(_, value)| *value as i64 / STROOP as i64)
                    .collect();
                let all_ledgers: Vec<String> = data
                    .collateral
                    .iter()
                    .map(|(ledger, _)| ledger.to_string())
                    .collect();

                dashboard = dashboard.add_line_chart(
                    "Collateral supply evolution",
                    all_ledgers,
                    vec![(
                        &format!("Collateral of pool {} and asset {}", pool, asset),
                        line_data,
                    )],
                );
            };

            {
                let line_data: Vec<i64> = data
                    .borrowed
                    .iter()
                    .map(|(_, value)| *value as i64 / STROOP as i64)
                    .collect();
                let all_ledgers: Vec<String> = data
                    .borrowed
                    .iter()
                    .map(|(ledger, _)| ledger.to_string())
                    .collect();

                dashboard = dashboard.add_line_chart(
                    "Borrow supply evolution",
                    all_ledgers,
                    vec![(
                        &format!("Borrowed of pool {} and asset {}", pool, asset),
                        line_data,
                    )],
                );
            };

            {
                dashboard = dashboard.add_table(
                    &format!("{} pool {} volume", pool, asset),
                    vec!["Timeframe".into(), "Volume".into()],
                    vec![
                        vec![
                            "24hrs".into(),
                            format!("{} {}", data.volume_24hrs as u64 / STROOP as u64, denom),
                        ],
                        vec![
                            "week".into(),
                            format!("{} {}", data.volume_week as u64 / STROOP as u64, denom),
                        ],
                        vec![
                            "month".into(),
                            format!("{} {}", data.volume_month as u64 / STROOP as u64, denom),
                        ],
                    ],
                )
            };
        }
    }

    {
        let mut rows = Vec::new();
        for entry in borroweds.iter().rev() {
            let (kind, amount) = if entry.delta > 0 {
                ("borrow".into(), ((entry.delta as u128) as i64).to_string())
            } else {
                ("repay".into(), ((entry.delta as u128) as i64).to_string())
            };

            rows.push(vec![
                kind,
                entry.timestamp.to_string(),
                entry.ledger.to_string(),
                entry.pool.to_string(),
                entry.asset.to_string(),
                entry.source.to_string(),
                amount,
            ]);
        }

        dashboard = dashboard.add_table(
            "Borrow Actions",
            vec![
                "type".into(),
                "timestamp".into(),
                "ledger".into(),
                "pool".into(),
                "asset".into(),
                "source".into(),
                "amount".into(),
            ],
            rows,
        );
    };

    {
        let mut rows = Vec::new();
        for entry in collaterals.iter().rev() {
            let (kind, amount) = if entry.delta > 0 {
                ("supply".into(), ((entry.delta as u128) as i64).to_string())
            } else {
                (
                    "withdraw".into(),
                    ((entry.delta as u128) as i64).to_string(),
                )
            };

            rows.push(vec![
                kind,
                entry.timestamp.to_string(),
                entry.ledger.to_string(),
                entry.pool.to_string(),
                entry.asset.to_string(),
                entry.source.to_string(),
                amount,
            ]);
        }
        dashboard = dashboard.add_table(
            "Last Collateral Actions",
            vec![
                "type".into(),
                "timestamp".into(),
                "ledger".into(),
                "pool".into(),
                "asset".into(),
                "source".into(),
                "amount".into(),
            ],
            rows,
        );
    };

    env.log().debug("collateral table built", None);

    dashboard.build()
}
