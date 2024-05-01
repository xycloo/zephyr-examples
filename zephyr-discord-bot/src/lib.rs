use zephyr_sdk::{
    soroban_sdk::{xdr::{ContractEvent, ContractEventBody, Hash, ScVal}, Symbol}, AgnosticRequest, EnvClient
};

const XYCLOANS_XLM_POOL: [u8; 32] = [
    34, 55, 84, 244, 68, 42, 17, 209, 68, 194, 133, 46, 96, 101, 43, 63, 89, 0, 232, 98, 133, 158,
    253, 185, 84, 188, 41, 254, 33, 38, 91, 21,
];

#[no_mangle]
pub extern "C" fn on_close() {
    let env = EnvClient::new();
    let contract_events: Vec<ContractEvent> = env
        .reader()
        .soroban_events()
        .into_iter()
        .filter(|event| event.contract_id == Some(Hash(XYCLOANS_XLM_POOL)))
        .collect();

    for event in contract_events {
        let ContractEventBody::V0(v0) = event.body;

        if env.from_scval::<Symbol>(&v0.topics[0]) == Symbol::new(&env.soroban(), "deposit")
            && env.from_scval::<i128>(&v0.data) >= 10_000_000_000
        {
            env.log().debug("got deposit larger than 1000 XLM", None);
            send_message(&env, v0.topics[1].clone(), v0.data)
        }
    }
}

fn send_message(env: &EnvClient, source: ScVal, amount: ScVal) {
    let source = {
        let ScVal::Address(address) = source else {
            panic!()
        };
        address.to_string()
    };

    let key = env!("DISCORD_API");
    let body = format!(
        r#"{{"content": "{}"}}"#,
        format!(
            "New large deposit of {:?} XLM from {} on xycLoans testnet XLM pool.",
            amount, source
        )
    );

    env.log()
        .debug(format!("sending with body {:?}", body), None);

    env.send_web_request(AgnosticRequest {
        body: Some(body),
        url: "https://discordapp.com/api/channels/1234475897092968459/messages".into(),
        method: zephyr_sdk::Method::Post,
        headers: vec![
            ("Content-Type".into(), "application/json".into()),
            ("Authorization".into(), format!("Bot {}", key)),
        ],
    })
}
