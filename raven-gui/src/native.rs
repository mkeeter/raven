use anyhow::{anyhow, Context};
use std::{io::Read, sync::mpsc};

use uxn::{Backend, Uxn, UxnRam};
use varvara::Varvara;

use anyhow::Result;
use eframe::egui;
use log::info;

use clap::Parser;

use crate::{audio_setup, Stage};

/// Uxn runner
#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// ROM to load and execute
    rom: std::path::PathBuf,

    /// Scale factor for the window
    #[clap(long)]
    scale: Option<f32>,

    /// Use the native assembly Uxn implementation
    #[clap(long)]
    native: bool,

    /// Arguments to pass into the VM
    #[arg(last = true)]
    args: Vec<String>,
}

pub fn run() -> Result<()> {
    let env = env_logger::Env::default()
        .filter_or("UXN_LOG", "info")
        .write_style_or("UXN_LOG", "always");
    env_logger::init_from_env(env);

    let args = Args::parse();
    let mut f = std::fs::File::open(&args.rom)
        .with_context(|| format!("failed to open {:?}", args.rom))?;

    let mut rom = vec![];
    f.read_to_end(&mut rom).context("failed to read file")?;

    let ram = UxnRam::new();
    let mut vm = Uxn::new(
        ram.leak(),
        if args.native {
            #[cfg(not(target_arch = "aarch64"))]
            anyhow::bail!("no native implementation for this arch");

            #[cfg(target_arch = "aarch64")]
            Backend::Native
        } else {
            Backend::Interpreter
        },
    );
    let mut dev = Varvara::new();
    let extra = vm.reset(&rom);
    dev.reset(extra);

    let _audio = audio_setup(dev.audio_streams());

    // Run the reset vector
    let start = std::time::Instant::now();
    vm.run(&mut dev, 0x100);
    info!("startup complete in {:?}", start.elapsed());

    dev.output(&vm).check()?;
    dev.send_args(&mut vm, &args.args).check()?;

    let (width, height) = dev.output(&vm).size;
    let options = eframe::NativeOptions {
        window_builder: Some(Box::new(move |v| {
            v.with_inner_size(egui::Vec2::new(width as f32, height as f32))
                .with_resizable(false)
        })),
        ..Default::default()
    };

    let (_tx, rx) = mpsc::channel();
    eframe::run_native(
        "Varvara",
        options,
        Box::new(move |cc| {
            Box::new(Stage::new(vm, dev, args.scale, rx, &cc.egui_ctx))
        }),
    )
    .map_err(|e| anyhow!("got egui error: {e:?}"))
}
