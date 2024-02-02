use rs_zephyr_sdk::{
    scval_utils,
    stellar_xdr::next::{
        ContractEvent, ContractEventBody, ContractExecutable, LedgerEntryData, Limits, ReadXdr,
        ScAddress, ScVal, TransactionMeta, WriteXdr,
    },
    EntryChanges, EnvClient,
};
use std::convert::TryInto;

fn to_array<T, const N: usize>(v: Vec<T>) -> [T; N] {
    v.try_into()
        .unwrap_or_else(|v: Vec<T>| panic!("Expected a Vec of length {} but it was {}", N, v.len()))
}

const POOL_HASH: [u8; 32] = [
    157, 26, 248, 13, 179, 177, 103, 229, 225, 217, 93, 115, 139, 231, 31, 231, 21, 107, 67, 177,
    23, 67, 28, 64, 96, 169, 26, 244, 92, 134, 198, 142,
];

pub enum ZephyrError {
    ContractNotAPool,
}

#[no_mangle]
pub extern "C" fn on_close() {
    let env = EnvClient::new();
    let reader = env.reader();

    let processing =
        ProcessingHandler::new(reader.ledger_timestamp(), reader.ledger_sequence(), &env);
    processing.run();
}

struct ProcessingHandler<'a> {
    ledger: u32,
    timestamp: u64,
    env: &'a EnvClient,
}

impl<'a> ProcessingHandler<'a> {
    fn new(timestamp: u64, ledger: u32, env: &'a EnvClient) -> Self {
        Self {
            timestamp,
            ledger,
            env,
        }
    }

    fn run(&self) {
        self.search_created();

        for result in self.env.reader().tx_processing() {
            if let TransactionMeta::V3(meta) = result.tx_apply_processing {
                if let Some(soroban) = meta.soroban_meta {
                    for event in soroban.events.iter() {
                        // No need to handle this, only error is if the event is not of a pool contract.
                        // In such case, we do nothing.
                        let _ = self.handle_event_with_db(event);
                    }
                }
            }
        }
    }

    fn write_supply(&self, contract_id: &[u8; 32], supply: i128) {
        self.env
            .db_write(
                "xlsupply",
                &["contract", "timestamp", "supply"],
                &[
                    contract_id,
                    &self.timestamp.to_be_bytes(),
                    &supply.to_be_bytes(),
                ],
            )
            .unwrap()
    }

    fn write_balance(&self, contract_id: &[u8; 32], address: &ScAddress, balance: i128) {
        self.env
            .db_write(
                "xlbalance",
                &["contract", "address", "timestamp", "balance"],
                &[
                    contract_id,
                    &address.to_xdr(Limits::none()).unwrap(),
                    &self.timestamp.to_be_bytes(),
                    &balance.to_be_bytes(),
                ],
            )
            .unwrap();
    }

    fn get_current_supply(&self, contract_id: [u8; 32]) -> Result<i128, ZephyrError> {
        let supply_growth = self
            .env
            .db_read("xlsupply", &["contract", "supply"])
            .unwrap();

        if !supply_growth.rows.is_empty() {
            let mut pool_growth = Vec::new();
            for row in supply_growth.rows.iter() {
                if contract_id == row.row.get(0).unwrap().0.as_slice() {
                    let supply = &row.row.get(1).unwrap().0;
                    pool_growth.push(i128::from_be_bytes(to_array::<u8, 16>(supply.clone())))
                }
            }

            if let Some(last) = pool_growth.last() {
                Ok(*last)
            } else {
                Err(ZephyrError::ContractNotAPool)
            }
        } else {
            Err(ZephyrError::ContractNotAPool)
        }
    }

    fn get_current_balance(&self, contract_id: [u8; 32], address: &ScAddress) -> i128 {
        let balances = self
            .env
            .db_read("xlbalance", &["contract", "address", "balance"])
            .unwrap();
        if !balances.rows.is_empty() {
            let mut balance_growth = Vec::new();
            for row in balances.rows.iter() {
                let address_from_db =
                    ScAddress::from_xdr(row.row.get(1).unwrap().0.as_slice(), Limits::none())
                        .unwrap();
                if contract_id == row.row.get(0).unwrap().0.as_slice()
                    && *address == address_from_db
                {
                    let balance = &row.row.get(2).unwrap().0;
                    balance_growth.push(i128::from_be_bytes(to_array::<u8, 16>(balance.clone())))
                }
            }
            *balance_growth.last().unwrap_or(&0)
        } else {
            0
        }
    }

    fn search_created(&self) {
        let EntryChanges { created, .. } = self.env.reader().v1_success_ledger_entries();

        for entry in created {
            if let LedgerEntryData::ContractData(data) = entry.data {
                let contract_hash = match &data.contract {
                    ScAddress::Contract(hash) => hash.0,
                    _ => panic!(),
                };
                if let ScVal::ContractInstance(instance) = data.val {
                    if let ContractExecutable::Wasm(hash) = instance.executable {
                        if hash.0 == POOL_HASH {
                            self.write_supply(&contract_hash, 0);
                        }
                    }
                }
            }
        }
    }

    /// Takes an event and stores it in xycLoans events table.
    fn handle_event_with_db(&self, event: &ContractEvent) -> Result<(), ZephyrError> {
        let contract_id = event.contract_id.as_ref().unwrap().0;
        let current_supply = self.get_current_supply(contract_id)?;

        let (topics, data) = match &event.body {
            ContractEventBody::V0(v0) => {
                if let Some(topic1) = v0.topics.get(0) {
                    if topic1 == &scval_utils::to_scval_symbol("borrow").unwrap() {
                        if let ScVal::I128(parts) = &v0.data {
                            let amount = scval_utils::parts_to_i128(parts);
                            let fee = (amount as f64 * 0.08) / 100.0;
                            let yield_percentage = ((fee as f64) * 100.0) / current_supply as f64; // NB: this is safe assuming a correct
                                                                                                   // execution of the soroban vm.

                            self.env
                                .db_write(
                                    "xlpoolyld",
                                    &["contract", "timestamp", "yieldnorm", "yield"],
                                    &[
                                        &contract_id,
                                        &self.timestamp.to_be_bytes(),
                                        &yield_percentage.to_be_bytes(),
                                        &fee.to_be_bytes(),
                                    ],
                                )
                                .unwrap()
                        }
                    } else if topic1 == &scval_utils::to_scval_symbol("newfee").unwrap() {
                        if let Some(ScVal::Address(user_address)) = &v0.topics.get(1) {
                            let current_balance =
                                self.get_current_balance(contract_id, user_address);

                            if let ScVal::I128(parts) = &v0.data {
                                let amount = scval_utils::parts_to_i128(parts);
                                let yield_percentage =
                                    ((amount as f64) * 100.0) / current_balance as f64; // NB: this is safe assuming a correct
                                                                                        // execution of the soroban vm.

                                self.env
                                    .db_write(
                                        "xluseryld",
                                        &["contract", "address", "timestamp", "yieldnorm", "yield"],
                                        &[
                                            &contract_id,
                                            &user_address.to_xdr(Limits::none()).unwrap(),
                                            &self.timestamp.to_be_bytes(),
                                            &yield_percentage.to_be_bytes(),
                                            &amount.to_be_bytes(),
                                        ],
                                    )
                                    .unwrap()
                            }
                        }
                    } else if topic1 == &scval_utils::to_scval_symbol("deposit").unwrap() {
                        if let Some(ScVal::Address(user_address)) = &v0.topics.get(1) {
                            let current_balance =
                                self.get_current_balance(contract_id, user_address);

                            if let ScVal::I128(parts) = &v0.data {
                                let amount = scval_utils::parts_to_i128(parts);

                                self.write_balance(
                                    &contract_id,
                                    &user_address,
                                    current_balance + amount,
                                );
                                self.write_supply(&contract_id, current_supply + amount);
                            }
                        }
                    } else if topic1 == &scval_utils::to_scval_symbol("withdrawn").unwrap() {
                        if let Some(ScVal::Address(user_address)) = &v0.topics.get(1) {
                            let current_balance =
                                self.get_current_balance(contract_id, user_address);

                            if let ScVal::I128(parts) = &v0.data {
                                let amount = scval_utils::parts_to_i128(parts);

                                self.write_balance(
                                    &contract_id,
                                    &user_address,
                                    current_balance - amount,
                                );
                                self.write_supply(&contract_id, current_supply - amount);
                            }
                        }
                    }
                }
                (
                    v0.topics
                        .iter()
                        .map(|topic| topic.to_xdr(Limits::none()).unwrap())
                        .collect::<Vec<Vec<u8>>>(),
                    v0.data.to_xdr(Limits::none()).unwrap(),
                )
            }
        };

        self.env
            .db_write(
                "xlevents",
                &[
                    "sequence",
                    "timestamp",
                    "contract",
                    "topic1",
                    "topic2",
                    "topic3",
                    "topic4",
                    "data",
                ],
                &[
                    &self.ledger.to_be_bytes(),
                    &self.timestamp.to_be_bytes(),
                    &contract_id,
                    &topics.get(0).unwrap_or(&vec![]),
                    &topics.get(1).unwrap_or(&vec![]),
                    &topics.get(2).unwrap_or(&vec![]),
                    &topics.get(3).unwrap_or(&vec![]),
                    &data,
                ],
            )
            .unwrap();

        Ok(())
    }
}
