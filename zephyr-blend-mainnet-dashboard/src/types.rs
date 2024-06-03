
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
    user: Address,  // the Address whose assets are involved in the auction
    auct_type: u32, // the type of auction taking place
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


#[derive(DatabaseDerive)]
#[with_name("supply")]
pub struct Supply {
    pub timestamp: u64,
    pub ledger: u32,
    pub pool: String,
    pub asset: String,
    pub supply: i128,
    pub delta: i128,
    pub change_source: String,
}

#[derive(DatabaseDerive)]
#[with_name("clateral")]
pub struct Collateral {
    pub timestamp: u64,
    pub ledger: u32,
    pub pool: String,
    pub asset: String,
    pub clateral: i128,
    pub delta: i128,
    pub change_source: String,
}

#[derive(DatabaseDerive, Serialize)]
#[with_name("borrowed")]
pub struct Borrowed {
    pub timestamp: u64,
    pub ledger: u32,
    pub pool: String,
    pub asset: String,
    pub borrowed: i128,
    pub delta: i128,
    pub change_source: String,
}

#[derive(DatabaseDerive, Serialize)]
#[with_name("auction")]
pub struct Auction {
    pub timestamp: u64,
    pub ledger: u32,
    pub pool: String,
    pub asset: String,
    pub atype: String,
    pub amount: i128,
    pub change_source: String,
}

impl Auction {
    pub fn new(env: &EnvClient, pool: Address, asset: ScVal, amount: i128, change_source: Address, atype: String) -> Self {
        Self { 
            timestamp: env.reader().ledger_timestamp(), 
            ledger: env.reader().ledger_sequence(), 
            pool: crate::chart::soroban_string_to_string(env, pool.to_string()), 
            asset: crate::chart::soroban_string_to_string(env, env.from_scval::<Address>(&asset).to_string()), 
            atype, 
            amount, 
            change_source: crate::chart::soroban_string_to_string(env, change_source.to_string()) 
        }
    }
}

pub(crate) trait Common {
    fn get_info(&self) -> (String, String, i128);

    fn new(env: &EnvClient, pool: Address, asset: ScVal, supply: i128, delta: i128, change_source: ScVal) -> Self;
}

macro_rules! impl_common {
    ($struct_name:ident, $denom:ident) => {
        impl Common for $struct_name {
            fn get_info(&self) -> (String, String, i128) {
                (self.pool.clone(), self.asset.clone(), self.$denom.clone())
            }

            fn new(env: &EnvClient, pool: Address, asset: ScVal, supply: i128, delta: i128, change_source: ScVal) -> Self {
                Self {
                    timestamp: env.reader().ledger_timestamp(),
                    ledger: env.reader().ledger_sequence(),
                    pool: crate::chart::soroban_string_to_string(env, pool.to_string()),
                    asset: crate::chart::soroban_string_to_string(env, env.from_scval::<Address>(&asset).to_string()),
                    change_source: crate::chart::soroban_string_to_string(env, env.from_scval::<Address>(&change_source).to_string()),
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
