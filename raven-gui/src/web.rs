use anyhow::{anyhow, Result};
use eframe::{
    egui,
    wasm_bindgen::{closure::Closure, JsCast},
    web_sys,
};
use log::{info, warn};
use std::sync::mpsc;

use crate::{audio_setup, Event, Stage};
use uxn::{Backend, Uxn, UxnRam};
use varvara::Varvara;

pub fn run() -> Result<()> {
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let ram = UxnRam::new();
    let rom = include_bytes!("../../roms/controller.rom");
    let mut vm = Uxn::new(rom, ram.leak(), Backend::Interpreter);
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

    let sel = document
        .get_element_by_id("example-selector")
        .ok_or_else(|| anyhow!("could not find example-selector"))?
        .dyn_into::<web_sys::Node>()
        .map_err(|e| anyhow!("could not convert example-selector: {e:?}"))?;

    const ROMS: &[(&'static str, &'static [u8])] = &[
        ("controller", include_bytes!("../../roms/controller.rom")),
        ("screen", include_bytes!("../../roms/screen.rom")),
        ("drool", include_bytes!("../../roms/drool.rom")),
        ("audio", include_bytes!("../../roms/audio.rom")),
        ("mandelbrot", include_bytes!("../../roms/mandelbrot.rom")),
        ("bunnymark", include_bytes!("../../roms/bunnymark.rom")),
        ("piano", include_bytes!("../../roms/piano.rom")),
    ];
    for (r, _) in ROMS {
        let opt = document
            .create_element("option")
            .map_err(|e| anyhow!("could not create option: {e:?}"))?
            .dyn_into::<web_sys::HtmlOptionElement>()
            .map_err(|e| {
                anyhow!("could not convert example-selector: {e:?}")
            })?;
        opt.set_text_content(Some(r));
        sel.append_child(&opt.get_root_node())
            .map_err(|e| anyhow!("could not append node: {e:?}"))?;
    }

    let (tx, rx) = mpsc::channel();
    let sel = document
        .get_element_by_id("example-selector")
        .ok_or_else(|| anyhow!("could not find example-selector"))?
        .dyn_into::<web_sys::HtmlSelectElement>()
        .map_err(|e| anyhow!("could not convert example-selector: {e:?}"))?;

    let a = Closure::<dyn FnMut()>::new(move || match sel.selected_index() {
        0 => (),
        i => {
            if let Some((_, r)) = ROMS.get(i as usize - 1) {
                if tx.send(Event::LoadRom(r.to_vec())).is_err() {
                    warn!("error loading rom");
                }
            } else {
                warn!("invalid selection: {i}");
            }
        }
    });
    let sel = document
        .get_element_by_id("example-selector")
        .ok_or_else(|| anyhow!("could not find example-selector"))?
        .dyn_into::<web_sys::HtmlSelectElement>()
        .map_err(|e| anyhow!("could not convert example-selector: {e:?}"))?;
    sel.set_onchange(Some(a.as_ref().unchecked_ref()));
    std::mem::forget(a);

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

    let resize_closure = Box::new(move |width: u16, height: u16| {
        div.style()
            .set_css_text(&format!("width: {width}px; height: {height}px"));
    });

    wasm_bindgen_futures::spawn_local(async {
        eframe::WebRunner::new()
            .start(
                "varvara",
                options,
                Box::new(move |cc| {
                    let mut s =
                        Box::new(Stage::new(vm, dev, None, rx, &cc.egui_ctx));
                    s.set_resize_callback(resize_closure);
                    s
                }),
            )
            .await
            .expect("failed to start eframe")
    });

    Ok(())
}
