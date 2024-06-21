use log::{error, info};

mod common;

#[cfg_attr(target_arch = "wasm32", path = "web.rs")]
#[cfg_attr(not(target_arch = "wasm32"), path = "native.rs")]
mod core;

fn main() -> anyhow::Result<()> {
    let out = core::run();
    match &out {
        Ok(()) => info!("core::run() completed successfully"),
        Err(e) => error!("core::run() failed: {e:?}"),
    };
    out
}
