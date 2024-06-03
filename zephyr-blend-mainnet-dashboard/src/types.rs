
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
    pub ledger: ScVal,
    pub pool: ScVal,
    pub asset: ScVal,
    pub supply: ScVal,
}

#[derive(DatabaseDerive)]
#[with_name("clateral")]
pub struct Collateral {
    pub ledger: ScVal,
    pub pool: ScVal,
    pub asset: ScVal,
    pub clateral: ScVal,
}

#[derive(DatabaseDerive, Serialize)]
#[with_name("borrowed")]
pub struct Borrowed {
    pub ledger: ScVal,
    pub pool: ScVal,
    pub asset: ScVal,
    pub borrowed: ScVal,
}


pub(crate) trait Common {
    fn get_info(&self) -> (ScVal, ScVal, ScVal);

    fn new(env: &EnvClient, pool: Address, asset: ScVal, supply: i128) -> Self;
}

macro_rules! impl_common {
    ($struct_name:ident, $denom:ident) => {
        impl Common for $struct_name {
            fn get_info(&self) -> (ScVal, ScVal, ScVal) {
                (self.pool.clone(), self.asset.clone(), self.$denom.clone())
            }

            fn new(env: &EnvClient, pool: Address, asset: ScVal, supply: i128) -> Self {
                Self {
                    ledger: env.to_scval(env.reader().ledger_sequence()),
                    pool: env.to_scval(pool),
                    asset,
                    $denom: env.to_scval(supply),
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
