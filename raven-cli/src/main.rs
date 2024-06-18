use std::io::Read;
use std::path::PathBuf;

use uxn::{Uxn, UxnRam};
use varvara::Varvara;

use anyhow::{Context, Result};
use clap::Parser;

/// Uxn runner
#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
    rom: PathBuf,
}

fn main() -> Result<()> {
    let env = env_logger::Env::default()
        .filter_or("UXN_LOG", "info")
        .write_style_or("UXN_LOG", "always");
    env_logger::init_from_env(env);

    let args = Args::parse();
    let mut f = std::fs::File::open(&args.rom)
        .with_context(|| format!("failed to open {:?}", args.rom))?;

    let mut rom = vec![];
    f.read_to_end(&mut rom).context("failed to read file")?;

    let mut ram = UxnRam::new();
    let mut vm = Uxn::new(&rom, &mut ram);
    let mut dev = Varvara::new();

    // Run the reset vector
    vm.run(&mut dev, 0x100);

    let out = dev.output(&vm);
    out.print()?;
    if let Some(e) = out.exit {
        std::process::exit(e);
    }

    // Blocking loop, listening to the stdin reader thread
    let rx = varvara::console_worker();
    while let Ok(c) = rx.recv() {
        let i = varvara::Input {
            console: Some(c),
            ..Default::default()
        };
        let out = dev.update(&mut vm, i);
        out.print()?;
        if let Some(e) = out.exit {
            std::process::exit(e);
        }
    }

    Ok(())
}
