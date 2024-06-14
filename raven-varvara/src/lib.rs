//! The Varvara computer system
#![warn(missing_docs)]
use log::warn;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

mod console;
mod datetime;
mod file;
mod system;

#[cfg(feature = "gui")]
mod screen;

#[cfg(feature = "gui")]
mod mouse;

#[cfg(feature = "gui")]
mod window;

#[cfg(feature = "gui")]
mod controller;

/// Audio handler implementation
pub mod audio;

use uxn::{Device, Ports, Uxn};

struct Event {
    /// Tuple of `(address, value)` to write in in device memory
    pub data: Option<(u8, u8)>,

    /// Vector to trigger
    pub vector: u16,
}

/// Handle to the Varvara system
pub struct Varvara {
    system: system::System,
    console: console::Console,
    datetime: datetime::Datetime,
    audio: audio::Audio,

    #[cfg(feature = "gui")]
    window: window::Window,

    /// Flags indicating if we've already printed a warning about a missing dev
    already_warned: [bool; 16],

    queue: VecDeque<Event>,
}

impl Default for Varvara {
    fn default() -> Self {
        Self::new()
    }
}

impl Device for Varvara {
    fn deo(&mut self, vm: &mut Uxn, target: u8) {
        match target & 0xF0 {
            system::SystemPorts::BASE => self.system.deo(vm, target),
            console::ConsolePorts::BASE => self.console.deo(vm, target),
            datetime::DatetimePorts::BASE => self.datetime.deo(vm, target),
            a if audio::AudioPorts::matches(a) => self.audio.deo(vm, target),

            #[cfg(feature = "gui")]
            _ if self.window.deo(vm, target) => (), // window handler

            // Default case
            t => self.warn_missing(t),
        }
    }
    fn dei(&mut self, vm: &mut Uxn, target: u8) {
        match target & 0xF0 {
            system::SystemPorts::BASE => self.system.dei(vm, target),
            console::ConsolePorts::BASE => self.console.dei(vm, target),
            datetime::DatetimePorts::BASE => self.datetime.dei(vm, target),
            a if audio::AudioPorts::matches(a) => self.audio.dei(vm, target),

            #[cfg(feature = "gui")]
            _ if self.window.dei(vm, target) => (), // window handler

            // Default case
            t => self.warn_missing(t),
        }
    }
}

impl Varvara {
    /// Builds a new instance of the Varvara peripherals
    pub fn new() -> Self {
        Self {
            console: console::Console::new(),
            system: system::System::default(),
            datetime: datetime::Datetime,
            audio: audio::Audio::new(),
            #[cfg(feature = "gui")]
            window: window::Window::new(),

            queue: VecDeque::with_capacity(1),
            already_warned: [false; 16],
        }
    }

    fn warn_missing(&mut self, t: u8) {
        if !self.already_warned[(t >> 4) as usize] {
            warn!("unimplemented device {t:#02x}");
            self.already_warned[(t >> 4) as usize] = true;
        }
    }

    /// Runs in a wait-loop
    #[cfg(feature = "gui")]
    pub fn run(&mut self, vm: &mut Uxn) {
        while self.window.redraw(vm) {
            self.queue.extend(self.console.poll(vm));
            self.window.update(vm, &mut self.queue);
            self.audio.update(vm, &mut self.queue);
            self.process_events(vm);
        }
    }

    /// Runs in a loop that blocks on console events
    #[cfg(not(feature = "gui"))]
    pub fn run(&mut self, vm: &mut Uxn) {
        loop {
            self.queue.extend(self.console.block(vm));
            self.process_events(vm);
        }
    }

    fn process_events(&mut self, vm: &mut Uxn) {
        while let Some(e) = self.queue.pop_front() {
            if let Some((addr, data)) = e.data {
                vm.write_dev_mem(addr, data);
            }
            vm.run(self, e.vector);
        }
    }

    /// Returns a handle to the given audio stream data
    ///
    /// # Panics
    /// There are only four audio streams, so this function panics if `i >= 4`
    pub fn audio_stream(&self, i: usize) -> Arc<Mutex<audio::StreamData>> {
        self.audio.stream(i)
    }
}
