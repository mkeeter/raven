use std::io::Read;
use std::path::PathBuf;

use raven_cli as cli;
use uxn::{Uxn, UxnMem, backend};
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

    /// Interpreter backend
    #[clap(long, default_value_t = Default::default())]
    backend: cli::Backend,

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

    match args.backend {
        cli::Backend::Interpreter => {
            run_with_backend::<backend::Interpreter>(&rom, &args)
        }
        #[cfg(feature = "native")]
        cli::Backend::Native => {
            run_with_backend::<backend::Native>(&rom, &args)
        }
        #[cfg(feature = "tailcall")]
        cli::Backend::Tailcall => {
            run_with_backend::<backend::Tailcall>(&rom, &args)
        }
    }
}

fn run_with_backend<B: uxn::Backend>(rom: &[u8], args: &Args) -> Result<()> {
    let mut mem = UxnMem::boxed();
    let mut vm = Uxn::<B>::new(&mut mem);
    let mut dev = Varvara::new();
    let data = vm.reset(rom);
    dev.reset(data);
    dev.init_args(&mut vm, &args.args);

    // Run the reset vector
    let start = std::time::Instant::now();
    vm.run(&mut dev, 0x100);
    info!("startup complete in {:?}", start.elapsed());

    dev.output(&vm).check()?;
    dev.send_args(&mut vm, &args.args).check()?;

    // Blocking loop, listening to the stdin reader thread
    let (tx, rx) = std::sync::mpsc::channel();
    varvara::spawn_console_worker(move |e| tx.send(e));
    while let Ok(c) = rx.recv() {
        dev.console(&mut vm, c);
        dev.output(&vm).check()?;
    }

    Ok(())
}
