
use serde::Serialize;
use zephyr_sdk::{
    prelude::*, soroban_sdk::{
        self, contracttype, xdr::ScVal, Address, Map, String as SorobanString
    }, DatabaseDerive, EnvClient
};

#[derive(Clone)]
#[contracttype]
pub struct UserReserveKey {
    user: Address,
    reserve_id: u32,
}

#[derive(Clone)]
#[contracttype]
pub struct AuctionKey {
    pub user: Address,  // the Address whose assets are involved in the auction
    pub auct_type: u32, // the type of auction taking place
}

#[derive(Clone)]
#[contracttype]
pub struct Positions {
    pub liabilities: Map<u32, i128>, // Map of Reserve Index to liability share balance
    pub collateral: Map<u32, i128>,  // Map of Reserve Index to collateral supply share balance
    pub supply: Map<u32, i128>,      // Map of Reserve Index to non-collateral supply share balance
}

#[derive(Clone)]
#[contracttype]
pub enum PoolDataKey {
    // A map of underlying asset's contract address to reserve config
    ResConfig(Address),
    // A map of underlying asset's contract address to queued reserve init
    ResInit(Address),
    // A map of underlying asset's contract address to reserve data
    ResData(Address),
    // The reserve's emission config
    EmisConfig(u32),
    // The reserve's emission data
    EmisData(u32),
    // Map of positions in the pool for a user
    Positions(Address),
    // The emission information for a reserve asset for a user
    UserEmis(UserReserveKey),
    // The auction's data
    Auction(AuctionKey),
    // A list of auctions and their associated data
    AuctData(Address),
}

#[derive(Clone)]
#[contracttype]
pub struct AuctionData {
    pub bid: Map<Address, i128>,
    pub lot: Map<Address, i128>,
    pub block: u32,
}

#[derive(Clone)]
#[contracttype]
pub enum PoolFactoryDataKey {
    Contracts(Address),
}

#[derive(Clone)]
#[contracttype]
pub(crate) struct StellarAssetContractMetadata {
    pub decimal: u32,
    pub name: SorobanString,
    pub symbol: SorobanString,
}

#[derive(DatabaseDerive, Clone, Default)]
#[with_name("totact")]
pub struct TotalActions {
    pub pool: String,
    pub asset: String,
    pub supply: i64,
    pub clateral: i64,
    pub borrowed: i64,
    pub auction: i64,
}

impl TotalActions {
    fn get(env: &EnvClient, pool: String, asset: String) -> Self {
        let query = env.read_filter().column_equal_to("pool", pool.clone()).column_equal_to("asset", asset.clone()).read();
        let res: Option<&Self> = query.as_ref().unwrap().get(0);
        if let Some(res) = res {
            res.clone()
        } else {
            let mut default = TotalActions::default();
            default.asset = asset;
            default.pool = pool;
            default.put(env);

            default
        }
    }
}

#[derive(DatabaseDerive, Clone)]
#[with_name("supply")]
pub struct Supply {
    pub id: i64,
    pub timestamp: u64,
    pub ledger: u32,
    pub pool: String,
    pub asset: String,
    pub supply: i128,
    pub delta: i128,
    pub source: String,
}

#[derive(DatabaseDerive, Clone)]
#[with_name("clateral")]
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

#[derive(DatabaseDerive, Serialize, Clone)]
#[with_name("borrowed")]
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

pub fn auction_from_u32(value: u32) -> String {
    match value {
        0 => "UserLiquidation".into(),
        1 => "BadDebtAuction".into(),
        2 => "InterestAuction".into(),
        _ => panic!("Blend broke!")
    }
}

/*
Currently deprecating auctions.
We still need to find a good way to index these along with the Script3 team.

fn auction_from_u32(value: u32) -> String {
    match value {
        0 => "UserLiquidation".into(),
        1 => "BadDebtAuction".into(),
        2 => "InterestAuction".into(),
        _ => panic!("Blend broke!")
    }
}

#[derive(DatabaseDerive, Serialize)]
#[with_name("auction")]
pub struct Auction {
    pub id: i64,
    pub timestamp: u64,
    pub ledger: u32,
    pub pool: String,
    pub asset: String,
    pub atype: String,
    pub amount: i128,
    pub source: String,
}

impl Auction {
    pub fn new(env: &EnvClient, pool: Address, asset: ScVal, amount: i128, change_source: Address, atype: String) -> Self {
        let mut total = TotalActions::get(env);
        let current = total.auction;
        total.auction = current + 1;
        env.update().column_equal_to("auction", current).execute(&total).unwrap();

        Self {
            id: current + 1,
            timestamp: env.reader().ledger_timestamp(), 
            ledger: env.reader().ledger_sequence(), 
            pool: crate::chart::soroban_string_to_string(env, pool.to_string()), 
            asset: crate::chart::soroban_string_to_string(env, env.from_scval::<Address>(&asset).to_string()), 
            atype, 
            amount, 
            source: crate::chart::soroban_string_to_string(env, change_source.to_string()) 
        }
    }
}
*/

pub(crate) trait Common {
    fn current(env: &EnvClient, asset: String, pool: String) -> i128;

    fn get_info(&self) -> (String, String, i128);

    fn new(env: &EnvClient, pool: Address, asset: ScVal, supply: i128, delta: i128, change_source: ScVal) -> Self;
}

macro_rules! impl_common {
    ($struct_name:ident, $denom:ident) => {
        impl Common for $struct_name {
            fn current(env: &EnvClient, asset: String, pool: String) -> i128 {
                env.log().debug(format!("getting current actions"), None);
                let current = TotalActions::get(env, pool.clone(), asset.clone()).$denom;
                env.log().debug(format!("total current {}", current), None);
                let rows: Vec<Self> = env.read_filter().column_equal_to("asset", asset).column_equal_to("pool", pool).column_equal_to("id", current).read().unwrap();
                env.log().debug(format!("got rows"), None);

                if let Some(row) = rows.get(0) {
                    row.$denom
                } else {
                    0
                }
            }

            fn get_info(&self) -> (String, String, i128) {
                (self.pool.clone(), self.asset.clone(), self.$denom.clone())
            }

            fn new(env: &EnvClient, pool: Address, asset: ScVal, supply: i128, delta: i128, change_source: ScVal) -> Self {
                let pool = crate::chart::soroban_string_to_string(env, pool.to_string());
                let asset =  crate::chart::soroban_string_to_string(env, env.from_scval::<Address>(&asset).to_string());
                let mut total = TotalActions::get(env, pool.clone(), asset.clone());
                let current = total.$denom;
                total.$denom = current + 1;
                env.update().column_equal_to("pool", pool.clone()).column_equal_to("asset", asset.clone()).execute(&total).unwrap();

                Self {
                    id: current + 1,
                    timestamp: env.reader().ledger_timestamp(),
                    ledger: env.reader().ledger_sequence(),
                    pool,
                    asset,
                    source: crate::chart::soroban_string_to_string(env, env.from_scval::<Address>(&change_source).to_string()),
                    delta,
                    $denom: supply,
                }
            }
        }
    };
}


impl_common!(Supply, supply);
impl_common!(Collateral, clateral);
impl_common!(Borrowed, borrowed);


#[derive(Default, Serialize)]
pub struct AggregatedData {
    pub borrowed: Vec<(u32, i128)>,
    pub supplied: Vec<(u32, i128)>,
    pub collateral: Vec<(u32, i128)>,
    pub total_borrowed: i128,
    pub total_supply: i128,
    pub total_collateral: i128,
}

impl AggregatedData {
    pub fn new() -> Self {
        AggregatedData::default()
    }

    pub fn add_supply(&mut self, ledger: u32, value: i128) {
        if self.total_supply < value {
            self.total_supply = value
        };
        self.supplied.push((ledger, value))
    }

    pub fn add_collateral(&mut self, ledger: u32, value: i128) {
        if self.total_collateral < value {
            self.total_collateral = value
        };
        self.collateral.push((ledger, value))
    }

    pub fn add_borrowed(&mut self, ledger: u32, value: i128) {
        if self.total_borrowed < value {
            self.total_borrowed = value
        };
        self.borrowed.push((ledger, value))
    }
}
