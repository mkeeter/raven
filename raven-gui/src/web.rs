use anyhow::{anyhow, Result};
use eframe::{
    egui,
    wasm_bindgen::{closure::Closure, JsCast},
    web_sys,
};
use log::info;

use crate::common::{audio_setup, Stage};
use uxn::{Uxn, UxnRam};
use varvara::Varvara;

pub fn run() -> Result<()> {
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let rom = include_bytes!("../../roms/potato.rom");
    let ram = UxnRam::new();
    let mut vm = Uxn::new(rom, ram.leak());
    let mut dev = Varvara::new();

    // Run the reset vector
    vm.run(&mut dev, 0x100);
    dev.output(&vm).check()?;

    let (width, height) = dev.output(&vm).size;
    let options = eframe::WebOptions {
        max_size_points: egui::Vec2::new(width as f32, height as f32),
        ..eframe::WebOptions::default()
    };

    info!("setting size to {width}, {height}");
    let document = web_sys::window()
        .ok_or_else(|| anyhow!("could not get window"))?
        .document()
        .ok_or_else(|| anyhow!("could not get document"))?;
    let div = document
        .get_element_by_id("box")
        .ok_or_else(|| anyhow!("could not find box div"))?;
    let div = div
        .dyn_into::<web_sys::HtmlElement>()
        .map_err(|e| anyhow!("could not cast to HtmlElement: {e:?}"))?;
    div.style()
        .set_css_text(&format!("width: {width}px; height: {height}px"));

    let mut _audio = None;
    let mut audio_data = Some(dev.audio_streams());
    let a = Closure::<dyn FnMut()>::new(move || {
        if let Some(d) = audio_data.take() {
            info!("setting up audio");
            _audio = Some(audio_setup(d));

            let div = document
                .get_element_by_id("audio")
                .expect("could not get audio warning")
                .dyn_into::<web_sys::HtmlElement>()
                .expect("could not cast to HtmlElement");
            div.style().set_css_text("color: #aaa");
        }
    });
    div.set_onclick(Some(a.as_ref().unchecked_ref()));
    std::mem::forget(a);

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
