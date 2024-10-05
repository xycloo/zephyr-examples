use zephyr_sdk::{prelude::*, EnvClient, String};

#[derive(Serialize, Deserialize)]
pub struct ColorClient {
    color: u32,
    amount: u32,
}

#[derive(Serialize, Deserialize)]
pub struct ColorMintRequest {
    source: String,
    colors: Vec<ColorClient>,
}

#[no_mangle]
pub extern "C" fn simulate_color_mint() {
    let env = EnvClient::empty();
    let request: ColorMintRequest = env.read_request_body();
    let function_name = Symbol::new(&env.soroban(), "colors_mine");
    let source_addr = Address::from_string(&String::from_str(&env.soroban(), &request.source));
    let mut colors = Map::new(&env.soroban());
    for color in request.colors {
        colors.set(color.color, color.amount);
    }
    let resp = env.simulate_contract_call(
        request.source,
        CONTRACT_ADDRESS,
        function_name,
        vec![
            &env.soroban(),
            source_addr.into_val(env.soroban()),
            colors.into_val(env.soroban()),
            ().into_val(env.soroban()),
            ().into_val(env.soroban()),
        ],
    );
    env.conclude(resp.unwrap())
}
