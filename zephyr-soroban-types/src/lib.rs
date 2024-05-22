use zephyr_sdk::{prelude::*, soroban_sdk::Symbol, EnvClient};
use colorglyph::types::Glyph;

#[no_mangle]
pub extern "C" fn on_close() {
    let env = EnvClient::new();
    for event in env.reader().pretty().soroban_events() {
        let action: Symbol = env.from_scval(&event.topics[0]);
        // easily perform comparisons with SDK types.
        if action == Symbol::new(&env.soroban(), "glyph_mint") {
            // relying in the contract's own custom types.
            let nft: Glyph = env.from_scval(&event.data);
            if nft.width > 64 {
                // do something
            } else {
                // do something else
            }
        }
    }
}
