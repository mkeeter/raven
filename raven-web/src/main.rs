use uxn::{Uxn, UxnRam};
use varvara::Varvara;

use anyhow::Result;
use eframe::egui;

use raven_gui::{audio_setup, Stage};

fn main() -> Result<()> {
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let rom = include_bytes!("../../roms/audio.rom");
    let ram = UxnRam::new();
    let mut vm = Uxn::new(rom, ram.leak());
    let mut dev = Varvara::new();

    let _audio = audio_setup(&dev);

    // Run the reset vector
    vm.run(&mut dev, 0x100);

    dev.output(&vm).check()?;

    let (width, height) = dev.output(&vm).size;
    let options = eframe::WebOptions {
        max_size_points: egui::Vec2::new(width as f32, height as f32),
        ..eframe::WebOptions::default()
    };

    wasm_bindgen_futures::spawn_local(async {
        eframe::WebRunner::new()
            .start(
                "varvara",
                options,
                Box::new(move |cc| Box::new(Stage::new(vm, dev, &cc.egui_ctx))),
            )
            .await
            .expect("failed to start eframe")
    });
    Ok(())
}
