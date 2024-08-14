use std::collections::{BTreeMap, HashMap};

use charming_fork_zephyr::{
    component::{Axis, Legend},
    element::{AreaStyle, AxisType, Color, ColorStop, Label, Tooltip, Trigger},
    series::{Bar, Line},
    Chart,
};
use zephyr_blend_dashboards::{
    chart::{get_from_instance, soroban_string_to_string},
    types::{StellarAssetContractMetadata, Supply},
};
use zephyr_sdk::{
    charting::{Dashboard, DashboardEntry, Table},
    prelude::*,
    soroban_sdk::{
        xdr::{Hash, ScAddress, ScVal},
        String as SorobanString,
    },
    DatabaseDerive, EnvClient,
};

#[derive(DatabaseDerive, Clone)]
#[with_name("pairs")]
#[external("9")]
struct PairsTable {
    address: ScVal,
    token_a: ScVal,
    token_b: ScVal,
    reserve_a: ScVal,
    reserve_b: ScVal,
}

#[derive(DatabaseDerive, Clone)]
#[with_name("events")]
#[external("9")]
struct EventsTable {
    e_type: ScVal,
    token_a: ScVal,
    token_b: ScVal,
    amount_a: ScVal,
    amount_b: ScVal,
    account: ScVal,
    timestamp: ScVal,
}

#[derive(DatabaseDerive, Clone)]
#[with_name("clateral")]
#[external("8")]
pub struct Collateral {
    pub id: i64,
    pub timestamp: u64,
    pub ledger: u32,
    pub pool: String,
    pub asset: String,
    pub clateral: i128,
    pub delta: i128,
    pub source: String,
}

#[derive(DatabaseDerive, Clone)]
#[with_name("borrowed")]
#[external("8")]
pub struct Borrowed {
    pub id: i64,
    pub timestamp: u64,
    pub ledger: u32,
    pub pool: String,
    pub asset: String,
    pub borrowed: i128,
    pub delta: i128,
    pub source: String,
}

pub struct Volume([i64; 3]);

#[derive(Clone, Copy)]
pub enum Protocol {
    Blend,
    Soroswap,
}

impl Volume {
    pub fn add(&mut self, protocol: Protocol, amount: i128) {
        match protocol {
            Protocol::Blend => self.add_blend(amount),
            Protocol::Soroswap => self.add_soroswap(amount),
        }
    }

    pub fn add_soroswap(&mut self, amount: i128) {
        self.0[0] += ((amount as i64) / 10000000) as i64;
        self.0[1] += ((amount as i64) / 10000000) as i64;
    }

    pub fn add_blend(&mut self, amount: i128) {
        self.0[0] += ((amount as i64) / 10000000) as i64;
        self.0[2] += ((amount as i64) / 10000000) as i64;
    }
}

pub struct Tvl(Vec<(u64, [i64; 3])>);

impl Tvl {
    pub fn new() -> Self {
        Self(vec![])
    }

    pub fn add(&mut self, protocol: Protocol, amount: i128, timestamp: u64) {
        match protocol {
            Protocol::Blend => {
                let mut previous_total = self.0.last().unwrap_or(&(0, [0; 3])).clone();
                previous_total.0 = timestamp;
                previous_total.1[0] += ((amount as i64) / 10000000) as i64;
                previous_total.1[2] += ((amount as i64) / 10000000) as i64;

                self.0.push(previous_total);
            }
            Protocol::Soroswap => {
                let mut previous_total = self.0.last().unwrap_or(&(0, [0; 3])).clone();
                previous_total.0 = timestamp;
                previous_total.1[0] += ((amount as i64) / 10000000) as i64;
                previous_total.1[1] += ((amount as i64) / 10000000) as i64;

                self.0.push(previous_total);
            }
        }
    }
}

pub const DAY_TIMEFRAME: u64 = 86_400;
pub const WEEK_TIMEFRAME: u64 = DAY_TIMEFRAME * 7;
pub const MONTH_TIMEFRAME: u64 = DAY_TIMEFRAME * 30;

pub struct AssetAggregation {
    vol_24_hrs: Volume,
    vol_week: Volume,
    vol_month: Volume,
    total_volume: Volume,
    total_value: Tvl,
}

impl AssetAggregation {
    pub fn new() -> Self {
        Self {
            vol_24_hrs: Volume([0; 3]),
            vol_week: Volume([0; 3]),
            vol_month: Volume([0; 3]),
            total_volume: Volume([0; 3]),
            total_value: Tvl::new(),
        }
    }

    fn update_volumes(
        &mut self,
        current_timestamp: u64,
        protocol: Protocol,
        amount: i128,
        timestamp: u64,
    ) {
        self.total_volume.add(protocol, amount);

        if timestamp + DAY_TIMEFRAME > current_timestamp {
            self.vol_24_hrs.add(protocol, amount)
        }

        if timestamp + WEEK_TIMEFRAME > current_timestamp {
            self.vol_week.add(protocol, amount)
        }

        if timestamp + MONTH_TIMEFRAME > current_timestamp {
            self.vol_month.add(protocol, amount)
        }
    }

    pub fn add(
        &mut self,
        env: &EnvClient,
        current_timestamp: u64,
        soroswap_event: Option<(ScVal, ScVal, ScVal)>,
        borrowed: Option<Borrowed>,
        collateral: Option<Collateral>,
    ) {
        if let Some(event) = soroswap_event {
            let e_type = soroban_string_to_string(env, env.from_scval(&event.2));
            let amount: i128 = env.from_scval(&event.0);
            let timestamp: u64 = env.from_scval(&event.1);
            self.update_volumes(current_timestamp, Protocol::Soroswap, amount, timestamp);

            if e_type == "add" {
                self.total_value.add(Protocol::Soroswap, amount, timestamp)
            } else if e_type == "remove" {
                self.total_value.add(Protocol::Soroswap, -amount, timestamp)
            }
        }

        if let Some(borrowed) = borrowed {
            let amount: i128 = borrowed.delta.abs();
            let timestamp: u64 = borrowed.timestamp;
            self.update_volumes(current_timestamp, Protocol::Blend, amount, timestamp);
        }

        if let Some(collateral) = collateral {
            let amount: i128 = collateral.delta;
            let amount_abs: i128 = collateral.delta.abs();
            let timestamp: u64 = collateral.timestamp;
            self.update_volumes(current_timestamp, Protocol::Blend, amount_abs, timestamp);
            self.total_value.add(Protocol::Blend, amount, timestamp);
        }
    }
}

#[no_mangle]
pub extern "C" fn dashboard() {
    let env = EnvClient::empty();
    let blend_borrowed: Vec<Borrowed> = env.read();
    env.log().debug("got blend borrowed", None);
    let blend_collateral: Vec<Collateral> = env.read();
    env.log().debug("got blend clateral", None);
    let soroswap_events: Vec<EventsTable> = env.read();

    let mut map = HashMap::new();
    let current_timestamp = env.soroban().ledger().timestamp();

    for event in soroswap_events {
        map.entry(event.token_a)
            .or_insert_with(AssetAggregation::new)
            .add(
                &env,
                current_timestamp,
                Some((
                    event.amount_a,
                    event.timestamp.clone(),
                    event.e_type.clone(),
                )),
                None,
                None,
            );
        map.entry(event.token_b)
            .or_insert_with(AssetAggregation::new)
            .add(
                &env,
                current_timestamp,
                Some((
                    event.amount_b,
                    event.timestamp.clone(),
                    event.e_type.clone(),
                )),
                None,
                None,
            );
    }

    for borrow in blend_borrowed {
        let asset_scval = ScVal::Address(ScAddress::Contract(Hash(
            stellar_strkey::Contract::from_string(&borrow.asset)
                .unwrap()
                .0,
        )));
        map.entry(asset_scval)
            .or_insert_with(AssetAggregation::new)
            .add(&env, current_timestamp, None, Some(borrow), None)
    }

    for collateral in blend_collateral {
        let asset_scval = ScVal::Address(ScAddress::Contract(Hash(
            stellar_strkey::Contract::from_string(&collateral.asset)
                .unwrap()
                .0,
        )));
        map.entry(asset_scval)
            .or_insert_with(AssetAggregation::new)
            .add(&env, current_timestamp, None, None, Some(collateral))
    }

    env.log().debug("Building dashboard", None);
    let mut dashboard = Dashboard::new()
        .title(&"Soroban DeFi Explorer")
        .description(
            &"Explore asset volumes and historical TVL for Soroban's most used protocols.",
        );
    let mut volumes_table = Table::new().columns(vec![
        "Asset".into(),
        "Total".into(),
        "Month".into(),
        "Week".into(),
        "Day".into(),
    ]);
    let mut dashboard_entries = Vec::new();

    let reverse: Vec<(&ScVal, &AssetAggregation)> = map.iter().collect();
    for (asset_scval, aggregation) in reverse.iter().rev() {
        if aggregation.total_value.0.last().unwrap_or(&(0, [0; 3])).1[0] == 0 {
            continue;
        }

        let ScVal::Address(ScAddress::Contract(Hash(asset_bytes))) = asset_scval else {
            panic!()
        };
        let meta: StellarAssetContractMetadata = env.from_scval(&get_from_instance(
            &env,
            &stellar_strkey::Contract(asset_bytes.clone()).to_string(),
            "METADATA",
        ));
        let asset = soroban_string_to_string(&env, meta.symbol);

        volumes_table = volumes_table.row(vec![
            asset.clone(),
            format!(
                "Blend: {}, Soroswap: {}",
                aggregation.total_volume.0[2], aggregation.total_volume.0[1]
            ),
            format!(
                "Blend: {}, Soroswap: {}",
                aggregation.vol_month.0[2], aggregation.vol_month.0[1]
            ),
            format!(
                "Blend: {}, Soroswap: {}",
                aggregation.vol_week.0[2], aggregation.vol_week.0[1]
            ),
            format!(
                "Blend: {}, Soroswap: {}",
                aggregation.vol_24_hrs.0[2], aggregation.vol_24_hrs.0[1]
            ),
        ]);

        let bar = Chart::new()
            .legend(Legend::new().show(true))
            .tooltip(Tooltip::new().trigger(Trigger::Axis))
            .x_axis(
                Axis::new()
                    .type_(AxisType::Category)
                    .data(vec!["Total", "Blend", "Soroswap"]),
            )
            .y_axis(Axis::new().type_(AxisType::Value))
            .series(Bar::new().data(vec![
                aggregation.total_value.0.last().unwrap().1[0],
                aggregation.total_value.0.last().unwrap().1[2],
                aggregation.total_value.0.last().unwrap().1[1],
            ]));

        let history = {
            let line_data: Vec<i64> = aggregation
                .total_value
                .0
                .iter()
                .map(|(_, value)| value[0])
                .collect();
            let line_data_1: Vec<i64> = aggregation
                .total_value
                .0
                .iter()
                .map(|(_, value)| value[1])
                .collect();
            let line_data_2: Vec<i64> = aggregation
                .total_value
                .0
                .iter()
                .map(|(_, value)| value[2])
                .collect();
            let all_ledgers: Vec<String> = aggregation
                .total_value
                .0
                .iter()
                .map(|(ledger, _)| ledger.to_string())
                .collect();

            let chart = Chart::new()
                .legend(Legend::new().show(true).left("150px").top("3%"))
                .tooltip(Tooltip::new().trigger(Trigger::Axis))
                .x_axis(Axis::new().type_(AxisType::Category).data(all_ledgers))
                .y_axis(Axis::new().type_(AxisType::Value))
                .series(Line::new().name("Total").data(line_data).area_style(
                    AreaStyle::new().color(Color::LinearGradient {
                        x: 0,
                        y: 0,
                        x2: 0,
                        y2: 1,
                        color_stops: vec![
                            ColorStop::new(0, "rgb(84, 112, 198)"),
                            ColorStop::new(1, "rgb(79, 209, 242)"),
                        ],
                    }),
                ))
                .series(Line::new().name("Soroswap").data(line_data_1).area_style(
                    AreaStyle::new().color(Color::LinearGradient {
                        x: 0,
                        y: 0,
                        x2: 0,
                        y2: 1,
                        color_stops: vec![
                            ColorStop::new(0, "rgb(84, 112, 198)"),
                            ColorStop::new(1, "rgb(79, 209, 242)"),
                        ],
                    }),
                ))
                .series(Line::new().name("Blend").data(line_data_2).area_style(
                    AreaStyle::new().color(Color::LinearGradient {
                        x: 0,
                        y: 0,
                        x2: 0,
                        y2: 1,
                        color_stops: vec![
                            ColorStop::new(0, "rgb(84, 112, 198)"),
                            ColorStop::new(1, "rgb(79, 209, 242)"),
                        ],
                    }),
                ));

            chart
        };

        dashboard_entries.push(
            DashboardEntry::new()
                .title(format!("Asset {} TVL", asset))
                .chart(bar),
        );
        dashboard_entries.push(
            DashboardEntry::new()
                .title(format!("Asset {} TVL historical", asset))
                .chart(history),
        );
    }

    dashboard = dashboard.entry(
        DashboardEntry::new()
            .title(format!("Assets Volume"))
            .table(volumes_table),
    );
    for entry in dashboard_entries {
        dashboard = dashboard.entry(entry)
    }

    env.conclude(&dashboard)
}

#[test]
fn test() {
    let bytes = stellar_strkey::Contract::from_string(
        "CC7CDFY2VGDODJ7WPO3JIK2MXLOAXL4LRQCC43UJDBAIJ4SVFO3HNPOC",
    )
    .unwrap()
    .0;
    println!(
        "{:?}",
        ScAddress::Contract(Hash(bytes)).to_xdr_base64(Limits::none())
    )
}
