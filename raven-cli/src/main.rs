use std::io::Read;
use std::path::PathBuf;

use uxn::{Backend, Uxn, UxnRam};
use varvara::Varvara;

use anyhow::{Context, Result};
use clap::Parser;
use log::info;

/// Uxn runner
#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// ROM to load and execute
    rom: PathBuf,

    /// Use the JIT-accelerated Uxn implementation
    #[clap(long)]
    jit: bool,

    /// Arguments to pass into the VM
    #[arg(last = true)]
    args: Vec<String>,
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
    let mut vm = Uxn::new(
        &rom,
        &mut ram,
        if args.jit {
            #[cfg(not(target_arch = "aarch64"))]
            anyhow::bail!("no JIT compiler implemented for this arch");

            #[cfg(target_arch = "aarch64")]
            Backend::Jit
        } else {
            Backend::Interpreter
        },
    );
    let mut dev = Varvara::new();

    // Run the reset vector
    let start = std::time::Instant::now();
    vm.run(&mut dev, 0x100);
    info!("startup complete in {:?}", start.elapsed());

    dev.output(&vm).check()?;
    dev.send_args(&mut vm, &args.args).check()?;

    // Blocking loop, listening to the stdin reader thread
    let rx = varvara::console_worker();
    while let Ok(c) = rx.recv() {
        dev.console(&mut vm, c);
        dev.output(&vm).check()?;
    }

    Ok(())
}
