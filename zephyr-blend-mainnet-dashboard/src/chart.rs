use std::collections::HashMap;
use charming::{component::{Axis, Grid, Legend, Title}, element::{AreaStyle, AxisType, Color, ColorStop, Tooltip, Trigger}, series::{Bar, Line}, Chart};
use crate::types::{AggregatedData, Borrowed, Collateral, PoolDataKey, Positions, StellarAssetContractMetadata, Supply};
use zephyr_sdk::{
    soroban_sdk::{
        xdr::{LedgerEntryData, ScString, ScVal}, Address, String as SorobanString, Symbol
    }, utils, ContractDataEntry, EnvClient
};

pub const STROOP: i128 = 10_000_000;

fn soroban_string_to_string(env: &EnvClient, string: SorobanString) -> String {
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

fn get_from_ledger(env: &EnvClient, contract: String) {
    let entries = get_all_entries(env, contract);
    let mut all_positions: HashMap<String, Positions> = HashMap::new();
    
    for entry in entries {
        let LedgerEntryData::ContractData(data) = entry.entry.data else {panic!()};
        if let Ok(entry_key) = env.try_from_scval::<PoolDataKey>(&entry.key) {
            match entry_key {
                PoolDataKey::Positions(user) => {
                    all_positions.insert(soroban_string_to_string(env, user.to_string()), env.from_scval(&data.val));
                },

                _ => ()
            }
        }
    }
}

fn scval_to_i128(val: &ScVal) -> i128 {
    let ScVal::I128(parts) = val else {panic!()};
    utils::parts_to_i128(&parts)
}

fn scval_to_u32(val: &ScVal) -> u32 {
    let ScVal::U32(int) = val else {panic!()};
    *int
}

fn address_to_string(address: &ScVal) -> String {
    let ScVal::Address(addr) = address else {panic!()};
    addr.to_string()
}

pub fn aggregate_data(
    supplies: Vec<Supply>,
    collaterals: Vec<Collateral>,
    borroweds: Vec<Borrowed>,
) -> HashMap<String, HashMap<String, AggregatedData>> {
    let mut aggregated_data: HashMap<String, HashMap<String, AggregatedData>> = HashMap::new();

    for supply in supplies {
        let pool = supply.pool;  // Convert pool to string for hashmap key
        let asset = supply.asset;  // Convert asset to string for hashmap key
        let supply_value = scval_to_i128(&supply.supply);
        let ledger: u32 = scval_to_u32(&supply.ledger);

        aggregated_data
        .entry(address_to_string(&pool))
            .or_insert_with(HashMap::new)
            .entry(address_to_string(&asset))
            .or_insert_with(AggregatedData::new)
            .add_supply(ledger, supply_value);
    }

    for collateral in collaterals {
        let pool = collateral.pool;
        let asset = collateral.asset;
        let collateral_value: i128 = scval_to_i128(&collateral.clateral);
        let ledger: u32 = scval_to_u32(&collateral.ledger);

        aggregated_data
            .entry(address_to_string(&pool))
            .or_insert_with(HashMap::new)
            .entry(address_to_string(&asset))
            .or_insert_with(AggregatedData::new)
            .add_collateral(ledger, collateral_value)
    }

    for borrowed in borroweds {
        let pool = borrowed.pool;
        let asset = borrowed.asset;
        let borrowed_value: i128 = scval_to_i128(&borrowed.borrowed);
        let ledger: u32 = scval_to_u32(&borrowed.ledger);

        aggregated_data
            .entry(address_to_string(&pool))
            .or_insert_with(HashMap::new)
            .entry(address_to_string(&asset))
            .or_insert_with(AggregatedData::new)
            .add_borrowed(ledger, borrowed_value)
    }

    aggregated_data
}


pub fn create_chart(env: &EnvClient, aggregated_data: HashMap<String, HashMap<String, AggregatedData>>) -> Chart {
    let mut bars: Vec<Bar> = Vec::new();
    let categories: Vec<String> = vec!["Supply".into(), "Collateral".into(), "Borrowed".into()];
    let mut ledgers = Vec::new();
    let mut lines: Vec<Line> = Vec::new();

    for (pool, assets) in aggregated_data {
        let val = get_from_instance(env, pool, "Name");
        let ScVal::String(string) = val else {panic!()};
        let pool = string.to_utf8_string().unwrap();
        
        for (asset, data) in assets {
            let meta: StellarAssetContractMetadata = env.from_scval(&get_from_instance(env, asset, "METADATA"));
            let asset = soroban_string_to_string(env, meta.name);
            
            bars.push(
                Bar::new()
                    .name(format!("Pool: {}, Asset {}", pool, asset))
                    .data(vec![data.total_supply as i64 / STROOP as i64, data.total_collateral as i64 / STROOP as i64, data.total_borrowed as i64 / STROOP as i64]),
            );
           {
                let line_data: Vec<i64> = data.collateral.iter().map(|(_, value)| *value as i64 / STROOP as i64).collect();
                let all_ledgers: Vec<String> = data.collateral.iter().map(|(ledger, _)| ledger.to_string()).collect();
                lines.push(Line::new().name(format!("Collateral of pool {} and asset {}", pool, asset)).data(line_data).area_style(AreaStyle::new().color(Color::LinearGradient { x: 0, y: 0, x2: 0, y2: 1, color_stops: vec![ColorStop::new(0, "rgb(255, 158, 68)"), ColorStop::new(1, "rgb(255, 70, 131)")] })));
                ledgers.push(all_ledgers);
            }
            {
                let line_data: Vec<i64> = data.borrowed.iter().map(|(_, value)| *value as i64 / STROOP as i64).collect();
                let all_ledgers: Vec<String> = data.borrowed.iter().map(|(ledger, _)| ledger.to_string()).collect();
                lines.push(Line::new().name(format!("Borrowed from pool {} and asset {}", pool, asset)).data(line_data).area_style(AreaStyle::new().color(Color::LinearGradient { x: 0, y: 0, x2: 0, y2: 1, color_stops: vec![ColorStop::new(0, "rgb(104,95,255)"), ColorStop::new(1, "rgb(169,240,255)")] })));
                ledgers.push(all_ledgers);
            }
        }
    }
    
    let mut chart = Chart::new()
        .title(Title::new().text("Blend Mainnet Dashboard").left("150px"))
        .legend(Legend::new().show(true).left("150px").top("3%"))
        .tooltip(Tooltip::new().trigger(Trigger::Axis))
        .x_axis(Axis::new().type_(AxisType::Category).data(categories.clone()).grid_index(0))
        .y_axis(Axis::new().type_(AxisType::Value).grid_index(0))
        .grid(Grid::new().left("200px").top("300px").width("400px").height("400px"));
    
    let mut slot = 1;
    let mut slot_iter = 0;
     
    for (idx, line) in lines.into_iter().enumerate() {
        if slot == 2 {
            slot = 0;
            slot_iter += 1;
        }
        let top = 550 * slot_iter + 300;
        let grid_idx = idx as i64 + 1;
        chart = chart
        .grid(Grid::new().left(format!("{}px", 540 * slot + 200)).top(format!("{}px", top)).width("400px").height("400px"))
        .x_axis(Axis::new().type_(AxisType::Category).data(ledgers[idx].clone()).grid_index(grid_idx))
        .y_axis(Axis::new().type_(AxisType::Value).grid_index(grid_idx)).series(line.x_axis_index(grid_idx).y_axis_index(grid_idx));

        slot += 1;
    }

    for bar in bars {
        chart = chart.series(bar.x_axis_index(0).y_axis_index(0));
    }
    chart
}
