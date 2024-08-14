pub mod chart;
pub mod types;

use chart::{aggregate_data, build_dashboard, soroban_string_to_string, STROOP};
use types::{Borrowed, Collateral, Common, PoolFactoryDataKey, Supply};
use zephyr_sdk::{
    prelude::*, soroban_sdk::{
        xdr::{ContractEvent, ContractEventBody, ContractEventV0, Hash, ScSymbol, ScVal}, Address, String as SorobanString, Symbol
    }, EnvClient
};

pub(crate) const FACTORY_CONTRACT_ADDRESS: [u8; 32] = [178, 63, 18, 76, 113, 152, 251, 31, 74, 139, 184, 239, 196, 211, 3, 205, 58, 60, 182, 44, 2, 69, 194, 82, 254, 104, 175, 110, 187, 158, 108, 73];


fn address_from_string(env: &EnvClient, contract: Option<Hash>) -> Address {
    Address::from_string(&SorobanString::from_str(
        &env.soroban(),
        &stellar_strkey::Contract(contract.as_ref().unwrap().0).to_string(),
    ))
}

fn get_current_supply<T: DatabaseInteract + Common>(env: &EnvClient, event: &ContractEventV0, address: Address) -> i128 {
    let asset_topic_string = soroban_string_to_string(env, env.from_scval::<Address>(&event.topics[1]).to_string());
    let pool_address_string = soroban_string_to_string(env, address.to_string());

    env.log().debug("getting current supply", None);
    T::current(env, asset_topic_string, pool_address_string)
}

fn add_supply<T: DatabaseInteract + Common>(env: &EnvClient, event: &ContractEventV0, contract: Address, increase: bool) {
    let (amount, _): (i128, i128) = env.from_scval(&event.data);
    let new_supply = {
        let current_supply = get_current_supply::<T>(&env, &event, contract.clone());
        if increase {
            current_supply + amount
        } else {
            current_supply - amount
        }
    };
    let delta = if increase {
        amount
    } else {
        -amount
    };
    
    let supply = T::new(&env, contract, event.topics[1].clone(), new_supply, delta, event.topics[2].clone());
    env.put(&supply);
}


#[no_mangle]
pub extern "C" fn on_close() {
    let env = EnvClient::new();
        
    let pools = {
        let mut pools = Vec::new();
        let entries = env.read_contract_entries(FACTORY_CONTRACT_ADDRESS).unwrap();
        for entry in entries {
            if let Ok(entry) = env.try_from_scval(&entry.key) {
                let PoolFactoryDataKey::Contracts(address) = entry;
                pools.push(address)
            }
        }

        pools
    };

    env.log().debug(format!("Pools: {:?}", pools.len()), None);

    let events: Vec<ContractEvent> = env.reader().soroban_events().into_iter().filter(|x| {
        pools.contains(&address_from_string(&env, x.contract_id.clone()))
    }).collect();

    for t_event in events {
        let contract_address = address_from_string(&env, t_event.contract_id);
        let ContractEventBody::V0(event) = t_event.body;
        
        let action: Symbol = env.from_scval(&event.topics[0]);
        if action == Symbol::new(env.soroban(), "supply") {
            add_supply::<Supply>(&env, &event, contract_address, true);
        } else if action == Symbol::new(env.soroban(), "withdraw") {
            add_supply::<Supply>(&env, &event, contract_address, false);
        } else if action == Symbol::new(env.soroban(), "supply_collateral") {
            add_supply::<Collateral>(&env, &event, contract_address, true);
        } else if action == Symbol::new(env.soroban(), "withdraw_collateral") {
            add_supply::<Collateral>(&env, &event, contract_address, false);
        } else if action == Symbol::new(env.soroban(), "borrow") {
            add_supply::<Borrowed>(&env, &event, contract_address, true)
        } else if action == Symbol::new(env.soroban(), "repay") {
            add_supply::<Borrowed>(&env, &event, contract_address, false)
        }/* else if action == Symbol::new(env.soroban(), "fill_auction") {
            let auction_type_string = {
                let auction_type: u32 = env.from_scval(&event.topics[2]);
                auction_from_u32(auction_type)
            };
            let ScVal::Vec(Some(vec)) = &event.data else {panic!()};
            let source = env.from_scval(&vec.get(0).unwrap());
            let amount = env.from_scval(&vec.get(1).unwrap());
            let auction = Auction::new(&env, contract_address, event.topics[1].clone(), amount, source, auction_type_string);
            env.put(&auction);
        }*/
    }
    env.log().debug(format!("execution end"), None);
}


#[no_mangle]
pub extern "C" fn dasdhboard() {
    let env = EnvClient::empty();
    env.log().debug("Starting program", None);
    let dasboard = {
        //let supplies = env.read();
        //env.log().debug(format!("{:?}", env.to_scval((env.from_scval::<i128>(&supplies[0].supply) as i64 / STROOP as i64) as i128)), None);
        let collaterals = env.read();
        let borroweds = env.read();
        env.log().debug("Aggregating data", None);
        let timestamp = env.soroban().ledger().timestamp();
        env.log().debug(format!("Timestamp is {}", timestamp), None);
        let aggregated = aggregate_data(timestamp as i64, &collaterals, &borroweds);
        env.log().debug("Data aggregated", None);
        let dashboard = build_dashboard(&env, aggregated, &collaterals, &borroweds);

        env.log().debug("chart built", None);
        dashboard
    };
    
    env.log().debug(format!("{:?}", serde_json::to_string(&dasboard)), None);
    env.log().debug("concluding", None);
    env.conclude(&dasboard)
}