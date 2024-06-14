use std::io::Read;
use std::path::PathBuf;

use uxn::{Uxn, UxnRam};
use varvara::Varvara;

use anyhow::{Context, Result};
use clap::Parser;
use cpal::traits::StreamTrait;

/// Uxn runner
#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
    rom: PathBuf,
}

fn audio_setup(dev: &Varvara) -> (cpal::Device, [cpal::Stream; 4]) {
    use varvara::audio::{CHANNELS, SAMPLE_RATE};

    use cpal::traits::{DeviceTrait, HostTrait};
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("no output device available");
    let mut supported_configs_range = device
        .supported_output_configs()
        .expect("error while querying configs");

    let supported_config = supported_configs_range
        .find_map(|c| c.try_with_sample_rate(cpal::SampleRate(SAMPLE_RATE)))
        .filter(|c| c.channels() == CHANNELS)
        .expect("no supported config?");
    let config = supported_config.config();

    let streams = [0, 1, 2, 3].map(|i| {
        let d = dev.audio_stream(i);
        let stream = device
            .build_output_stream(
                &config,
                move |data: &mut [f32], _opt: &cpal::OutputCallbackInfo| {
                    d.lock().unwrap().next(data);
                },
                move |err| {
                    panic!("{err}");
                },
                None,
            )
            .expect("could not build stream");
        stream.play().unwrap();
        stream
    });
    (device, streams)
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

    let _audio = audio_setup(&dev);

    let start = std::time::Instant::now();
    vm.run(&mut dev, 0x100);
    println!("{:?}", start.elapsed());
    dev.run(&mut vm);

    Ok(())
}
