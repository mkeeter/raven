use anyhow::{anyhow, Result};
use eframe::{
    egui,
    wasm_bindgen::{closure::Closure, JsCast},
    web_sys,
};
use log::{error, info};
use std::sync::mpsc;
use wasm_bindgen_futures::JsFuture;
use web_sys::js_sys::Uint8Array;

use crate::{audio_setup, Event, Stage};
use uxn::{Backend, Uxn, UxnRam};
use varvara::Varvara;

pub fn run() -> Result<()> {
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let window =
        web_sys::window().ok_or_else(|| anyhow!("could not get window"))?;
    let loc = window.location();
    let hash = loc
        .hash()
        .map_err(|e| anyhow!("could not get location hash: {e:?}"))?;
    let rom_name = hash.strip_prefix('#');

    const ROMS: &[(&str, &[u8])] = &[
        ("controller", include_bytes!("../../roms/controller.rom")),
        ("screen", include_bytes!("../../roms/screen.rom")),
        ("drool", include_bytes!("../../roms/drool.rom")),
        ("audio", include_bytes!("../../roms/audio.rom")),
        ("mandelbrot", include_bytes!("../../roms/mandelbrot.rom")),
        ("bunnymark", include_bytes!("../../roms/bunnymark.rom")),
        ("piano", include_bytes!("../../roms/piano.rom")),
    ];

    let rom = ROMS
        .iter()
        .find(|(name, _data)| Some(*name) == rom_name)
        .map(|(_name, data)| *data)
        .unwrap_or(include_bytes!("../../roms/controller.rom"));

    let ram = UxnRam::new();
    let mut vm = Uxn::new(ram.leak(), Backend::Interpreter);
    let mut dev = Varvara::new();
    let extra = vm.reset(rom);
    dev.reset(extra);

    // Run the reset vector
    vm.run(&mut dev, 0x100);
    dev.output(&vm).check()?;

    let size @ (width, height) = dev.output(&vm).size;
    let options = eframe::WebOptions {
        max_size_points: egui::Vec2::new(width as f32, height as f32),
        ..eframe::WebOptions::default()
    };

    info!("setting size to {width}, {height}");
    let document = window
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
    let footer = document
        .get_element_by_id("footer")
        .ok_or_else(|| anyhow!("could not find footer div"))?;
    let footer = footer
        .dyn_into::<web_sys::HtmlElement>()
        .map_err(|e| anyhow!("could not cast to HtmlElement: {e:?}"))?;
    footer.style().set_css_text(&format!("width: {width}px"));

    let sel = document
        .get_element_by_id("example-selector")
        .ok_or_else(|| anyhow!("could not find example-selector"))?
        .dyn_into::<web_sys::Node>()
        .map_err(|e| anyhow!("could not convert example-selector: {e:?}"))?;

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

    let sel = document
        .get_element_by_id("example-selector")
        .ok_or_else(|| anyhow!("could not find example-selector"))?
        .dyn_into::<web_sys::HtmlSelectElement>()
        .map_err(|e| anyhow!("could not convert example-selector: {e:?}"))?;

    let (tx, rx) = mpsc::channel();
    let tx_ = tx.clone();
    let a = Closure::<dyn FnMut()>::new(move || match sel.selected_index() {
        0 => (),
        i => {
            if let Some((name, r)) = ROMS.get(i as usize - 1) {
                if tx_.send(Event::LoadRom(r.to_vec())).is_err() {
                    error!("error loading rom");
                }
                if let Err(e) = loc.set_hash(&format!("#{name}")) {
                    error!("could not update URL hash: {e:?}");
                }
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

    let file_load = document
        .get_element_by_id("load-file")
        .ok_or_else(|| anyhow!("could not find load-file"))?
        .dyn_into::<web_sys::HtmlInputElement>()
        .map_err(|e| anyhow!("could not convert load-file: {e:?}"))?;
    let tx_ = tx.clone();
    let a =
        Closure::<dyn FnMut(web_sys::Event)>::new(move |e: web_sys::Event| {
            let Some(t) = e.target() else {
                error!("could not get target from event");
                return;
            };
            let t = t.dyn_into::<web_sys::HtmlInputElement>().unwrap();
            let Some(f) = t.files() else {
                error!("could not get file list");
                return;
            };
            let Some(f) = f.item(0) else {
                error!("could not get file");
                return;
            };
            log::info!("got files {f:?}");
            let fut = JsFuture::from(f.array_buffer());
            let tx_ = tx_.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let v = fut.await;
                let v = match v {
                    Ok(v) => v,
                    Err(e) => {
                        error!("could not wait for future: {e:?}");
                        return;
                    }
                };
                let Ok(buf) = v.dyn_into::<web_sys::js_sys::ArrayBuffer>()
                else {
                    error!("could not cast to ArrayBuffer");
                    return;
                };
                let buf = Uint8Array::new(&buf);
                let mut dst = vec![0; buf.length() as usize];
                buf.copy_to(&mut dst);
                if tx_.send(Event::LoadRom(dst)).is_err() {
                    error!("error loading rom");
                }
                log::info!("got result {buf:?}");
            });
        });
    file_load.set_onchange(Some(a.as_ref().unchecked_ref()));
    std::mem::forget(a);

    let mut _audio = None;
    let mut audio_data = Some(dev.audio_streams());
    let audio_check = document
        .get_element_by_id("audio-check")
        .ok_or_else(|| anyhow!("could not find audio-check"))?
        .dyn_into::<web_sys::HtmlElement>()
        .map_err(|e| anyhow!("could not cast to HtmlElement: {e:?}"))?;
    let a = Closure::<dyn FnMut()>::new(move || {
        if let Some(d) = audio_data.take() {
            info!("setting up audio");
            _audio = audio_setup(d);
        }
        let audio_check = document
            .get_element_by_id("audio-check")
            .ok_or_else(|| anyhow!("could not find audio-check"))
            .unwrap()
            .dyn_into::<web_sys::HtmlInputElement>()
            .map_err(|e| anyhow!("could not cast to HtmlInputElement: {e:?}"))
            .unwrap();
        if tx.send(Event::SetMuted(!audio_check.checked())).is_err() {
            error!("error setting muted flag");
        }
    });
    audio_check.set_onclick(Some(a.as_ref().unchecked_ref()));
    std::mem::forget(a);

    let resize_closure = Box::new(move |width: u16, height: u16| {
        div.style()
            .set_css_text(&format!("width: {width}px; height: {height}px"));
        footer.style().set_css_text(&format!("width: {width}px"));
    });

    wasm_bindgen_futures::spawn_local(async move {
        eframe::WebRunner::new()
            .start(
                "varvara",
                options,
                Box::new(move |cc| {
                    let mut s = Box::new(Stage::new(
                        vm,
                        dev,
                        size,
                        1.0,
                        rx,
                        &cc.egui_ctx,
                    ));
                    s.set_resize_callback(resize_closure);
                    s
                }),
            )
            .await
            .expect("failed to start eframe")
    });

    Ok(())
}
