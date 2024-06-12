use std::{collections::{BTreeMap, HashMap}, ops::AddAssign};
use charming_fork_zephyr::{component::{Axis, Grid, Legend, Title}, element::{AreaStyle, AxisType, Color, ColorStop, Tooltip, Trigger}, series::{Bar, Line}, Chart};
use crate::types::{auction_from_u32, AggregatedData, AuctionData, AuctionKey, Borrowed, Collateral, PoolDataKey, Positions, StellarAssetContractMetadata, Supply};
use zephyr_sdk::{
    charting::{Dashboard, DashboardEntry, Table}, soroban_sdk::{
        xdr::{LedgerEntryData, ScString, ScVal}, Address, String as SorobanString, Symbol
    }, utils, ContractDataEntry, EnvClient
};

pub const STROOP: i128 = 10_000_000;
pub const DAY_TIMEFRAME: i64 = 86_400;

pub fn soroban_string_to_string(env: &EnvClient, string: SorobanString) -> String {
    let sc_val: ScVal = env.to_scval(string);
    if let ScVal::String(ScString(s)) = sc_val {
        let s = s.to_utf8_string().unwrap();
        let parts: Vec<&str> = s.split(':').collect();
        parts[0].into()
    } else {
        panic!("value is not a string");
    }
}

fn get_from_instance(env: &EnvClient, contract: String, str_: &str) -> ScVal {
    let instance = env.read_contract_instance(stellar_strkey::Contract::from_string(&contract).unwrap().0).unwrap().unwrap();
    let LedgerEntryData::ContractData(data) = instance.entry.data else {
        panic!()
    };
    let ScVal::ContractInstance(instance) = data.val else {panic!()};
    let val = instance.storage.as_ref().unwrap().0.iter().find(|entry| entry.key == env.to_scval(Symbol::new(&env.soroban(), str_)));

    val.unwrap().val.clone()
}

fn get_all_entries(env: &EnvClient, contract: String) -> Vec<ContractDataEntry> {
    env.read_contract_entries(stellar_strkey::Contract::from_string(&contract).unwrap().0).unwrap()
}

fn get_from_ledger(env: &EnvClient, contract: String) -> Vec<(AuctionKey, ScVal)> {
    let entries = get_all_entries(env, contract);
    //let mut all_positions: HashMap<String, Positions> = HashMap::new();
    let mut all_auctions = Vec::new();
    for entry in entries {
        let LedgerEntryData::ContractData(data) = entry.entry.data else {
            env.log().debug(format!("not contract data {:?}", entry.entry.data), None);
            panic!()};
        env.log().debug(format!("key {:?}", entry.key), None);
        if let Ok(entry_key) = env.try_from_scval::<PoolDataKey>(&data.key) {
            match entry_key {
                /*PoolDataKey::Positions(user) => {
                    all_positions.insert(soroban_string_to_string(env, user.to_string()), env.from_scval(&data.val));
                },*/

                PoolDataKey::Auction(key) => {
                    all_auctions.push((key, data.val.clone()))
                }

                _ => ()
            }
        }
    }

    all_auctions
}

pub fn aggregate_data(
    timestamp: i64,
    supplies: Vec<Supply>,
    collaterals: Vec<Collateral>,
    borroweds: Vec<Borrowed>,
) -> (HashMap<String, HashMap<String, AggregatedData>>, Vec<(String, u64)>) {
    let env = EnvClient::empty();
    let mut aggregated_data: HashMap<String, HashMap<String, AggregatedData>> = HashMap::new();
    let mut volume_24hrs: Vec<(String, u64)> = Vec::new();
    //let mut volume_week = HashMap::new();

    env.log().debug("hashmaps", None);

    for supply in supplies {
        let pool = supply.pool;  // Convert pool to string for hashmap key
        let asset = supply.asset;  // Convert asset to string for hashmap key
        let supply_value = supply.supply;
        let ledger =supply.ledger;

        aggregated_data
        .entry(pool)
            .or_insert_with(HashMap::new)
            .entry(asset)
            .or_insert_with(AggregatedData::new)
            .add_supply(ledger, supply_value);
    }

    for collateral in collaterals {
        let pool = collateral.pool;
        let asset = collateral.asset;
        let collateral_value = collateral.clateral;
        let ledger = collateral.ledger;

        let entry_timestamp = collateral.timestamp;
        if entry_timestamp as i64 + DAY_TIMEFRAME > timestamp {
            for (s_asset, s_volume) in volume_24hrs.iter_mut() {
                if asset == *s_asset {
                    *s_volume += collateral.delta as u64
                }
            }
        }

        aggregated_data
        .entry(pool)
        .or_insert_with(HashMap::new)
        .entry(asset)
        .or_insert_with(AggregatedData::new)
            .add_collateral(ledger, collateral_value)
    }

    for borrowed in borroweds {
        let pool = borrowed.pool;
        let asset = borrowed.asset;
        let borrowed_value = borrowed.borrowed;
        let ledger = borrowed.ledger;

        let entry_timestamp = borrowed.timestamp;
        if entry_timestamp as i64 + DAY_TIMEFRAME > timestamp {
            for (s_asset, s_volume) in volume_24hrs.iter_mut() {
                if asset == *s_asset {
                    *s_volume += borrowed.delta as u64
                }
            }
        }

        aggregated_data
        .entry(pool)
        .or_insert_with(HashMap::new)
        .entry(asset)
        .or_insert_with(AggregatedData::new)
            .add_borrowed(ledger, borrowed_value)
    }

    (aggregated_data, volume_24hrs)
}

pub fn build_dashboard(env: &EnvClient, aggregated_data: HashMap<String, HashMap<String, AggregatedData>>, volume: Vec<(String, u64)>, collaterals: Vec<Collateral>, borroweds: Vec<Borrowed>) -> Dashboard {
    let mut dashboard = Dashboard::new();
    let categories: Vec<String> = vec!["Supply".into(), "Collateral".into(), "Borrowed".into()];

    let volumes_table = {
        let mut table = Table::new().columns(vec!["Asset".into(), "Volume".into()]);

        for (asset, volume) in volume {
            let meta: StellarAssetContractMetadata = env.from_scval(&get_from_instance(env, asset, "METADATA"));
            let asset = soroban_string_to_string(env, meta.name);
            let denom = soroban_string_to_string(env, meta.symbol);

            table = table.row(vec![asset, format!("{} {}", volume / STROOP as u64, denom)]);
        }

        DashboardEntry::new().title("24 Hours Volume By Asset").table(table)
    };

    dashboard = dashboard.entry(volumes_table);

    for (pool, assets) in aggregated_data {
        let auctions_table = {
            let mut table = Table::new().columns(vec!["type".into(), "block".into(), "user".into(), "bid".into(), "lot".into()]);
            for entry in get_from_ledger(env, pool.clone()) {
                let user = entry.0.user;
                let atype = auction_from_u32(entry.0.auct_type);
                let data: AuctionData = env.from_scval(&entry.1);

                table = table.row(vec![atype, data.block.to_string(), soroban_string_to_string(env, user.to_string()), serde_json::to_string(&env.to_scval(data.bid)).unwrap(), serde_json::to_string(&env.to_scval(data.lot)).unwrap()]);
            }

            DashboardEntry::new().title("Current Auctions").table(table)
        };

        dashboard = dashboard.entry(auctions_table);

        let val = get_from_instance(env, pool, "Name");
        let ScVal::String(string) = val else {panic!()};
        let pool = string.to_utf8_string().unwrap();
        
        for (asset, data) in assets {
            let meta: StellarAssetContractMetadata = env.from_scval(&get_from_instance(env, asset, "METADATA"));
            let asset = soroban_string_to_string(env, meta.name);
            
            env.log().debug("got asset", None);

            let bar = {
                let chart = Chart::new().legend(Legend::new().show(true).left("150px").top("3%")).tooltip(Tooltip::new().trigger(Trigger::Axis))
                .x_axis(Axis::new().type_(AxisType::Category).data(categories.clone()))
                .y_axis(Axis::new().type_(AxisType::Value)).series(Bar::new()
                .name(format!("Pool: {}, Asset {}", pool, asset))
                .data(vec![data.total_supply as i64 / STROOP as i64, data.total_collateral as i64 / STROOP as i64, data.total_borrowed as i64 / STROOP as i64]));

                DashboardEntry::new().title("Distribution all time").chart(chart)
            };

            
            let collateral = {
                let line_data: Vec<i64> = data.collateral.iter().map(|(_, value)| *value as i64 / STROOP as i64).collect();
                let all_ledgers: Vec<String> = data.collateral.iter().map(|(ledger, _)| ledger.to_string()).collect();
                
                let chart = Chart::new().legend(Legend::new().show(true).left("150px").top("3%")).tooltip(Tooltip::new().trigger(Trigger::Axis))
                .x_axis(Axis::new().type_(AxisType::Category).data(all_ledgers))
                .y_axis(Axis::new().type_(AxisType::Value)).series(Line::new().name(format!("Collateral of pool {} and asset {}", pool, asset)).data(line_data).area_style(AreaStyle::new().color(Color::LinearGradient { x: 0, y: 0, x2: 0, y2: 1, color_stops: vec![ColorStop::new(0, "rgb(255, 158, 68)"), ColorStop::new(1, "rgb(255, 70, 131)")] })));

                DashboardEntry::new().title("Collateral supply evolution").chart(chart)
            };

            let borrowed = {
                let line_data: Vec<i64> = data.borrowed.iter().map(|(_, value)| *value as i64 / STROOP as i64).collect();
                let all_ledgers: Vec<String> = data.borrowed.iter().map(|(ledger, _)| ledger.to_string()).collect();
                
                let chart = Chart::new().legend(Legend::new().show(true).left("150px").top("3%")).tooltip(Tooltip::new().trigger(Trigger::Axis))
                .x_axis(Axis::new().type_(AxisType::Category).data(all_ledgers))
                .y_axis(Axis::new().type_(AxisType::Value)).series(Line::new().name(format!("Borrowed pool {} and asset {}", pool, asset)).data(line_data).area_style(AreaStyle::new().color(Color::LinearGradient { x: 0, y: 0, x2: 0, y2: 1, color_stops: vec![ColorStop::new(0, "rgb(255, 158, 68)"), ColorStop::new(1, "rgb(255, 70, 131)")] })));

                DashboardEntry::new().title("Borrwed supply evolution").chart(chart)
            };

            dashboard = dashboard.entry(bar).entry(collateral).entry(borrowed);
        }
    }

    let borrow_table = {
        let mut table = Table::new();
        table = table.columns(vec!["type".into(), "timestamp".into(), "ledger".into(), "pool".into(), "asset".into(), "source".into(), "amount".into()]);

        for entry in borroweds {
            let (kind, amount) = if entry.delta > 0 {
                ("borrow".into(), ((entry.delta as u128) as i64).to_string())
            } else {
                ("repay".into(), ((entry.delta as u128) as i64).to_string())
            };

            table = table.row(vec![kind, entry.timestamp.to_string(), entry.ledger.to_string(), entry.pool, entry.asset, entry.source, amount]);
        }
        
        let actions = DashboardEntry::new().title("Borrow Actions").table(table);
        actions
    };

    let collateral_table = {
        let mut table = Table::new();
        table = table.columns(vec!["type".into(), "timestamp".into(), "ledger".into(), "pool".into(), "asset".into(), "source".into(), "amount".into()]);

        for entry in collaterals {
            let (kind, amount) = if entry.delta > 0 {
                ("supply".into(), ((entry.delta as u128) as i64).to_string())
            } else {
                ("withdraw".into(), ((entry.delta as u128) as i64).to_string())
            };

            table = table.row(vec![kind, entry.timestamp.to_string(), entry.ledger.to_string(), entry.pool, entry.asset, entry.source, amount]);
        }
        
        let actions = DashboardEntry::new().title("Collateral Actions").table(table);
        actions
    };

    dashboard = dashboard.entry(borrow_table).entry(collateral_table);

    dashboard
}
