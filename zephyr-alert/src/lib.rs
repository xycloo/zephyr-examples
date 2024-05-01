use zephyr_sdk::{AgnosticRequest, EnvClient};

#[no_mangle]
pub extern "C" fn on_close() {
    let env = EnvClient::new();

    env.send_web_request(AgnosticRequest {
        body: Some("Hello from Zephyr Monitor Program!".into()),
        url: "https://tdep.requestcatcher.com/".into(),
        method: zephyr_sdk::Method::Get,
        headers: vec![("Custom".into(), "Header".into())]
    })
}
