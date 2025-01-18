//! The Varvara computer system
#![warn(missing_docs)]
use log::warn;
use std::{
    io::Write,
    sync::{Arc, Mutex},
};

mod console;
mod controller;
mod datetime;
mod file;
mod mouse;
mod screen;
mod system;

/// Audio handler implementation
mod audio;

pub use audio::StreamData;
pub use audio::CHANNELS as AUDIO_CHANNELS;
pub use audio::SAMPLE_RATE as AUDIO_SAMPLE_RATE;

pub use controller::Key;
pub use mouse::MouseState;

#[cfg(not(target_arch = "wasm32"))]
pub use console::worker as console_worker;

use uxn::{Device, Ports, Uxn};

/// Write to execute before calling the event vector
#[derive(Copy, Clone, Debug)]
struct EventData {
    addr: u8,
    value: u8,
    clear: bool,
}

/// Internal events, accumulated by devices then applied to the CPU
#[derive(Copy, Clone, Debug)]
struct Event {
    /// Tuple of `(address, value)` to write in in device memory
    pub data: Option<EventData>,

    /// Vector to trigger
    pub vector: u16,
}

/// Output from [`Varvara::update`], which may modify the GUI
pub struct Output<'a> {
    /// Current window size
    pub size: (u16, u16),

    /// Current screen contents, as RGBA values
    pub frame: &'a [u8],

    /// The system's mouse cursor should be hidden
    pub hide_mouse: bool,

    /// Outgoing console characters sent to the `write` port
    pub stdout: Vec<u8>,

    /// Outgoing console characters sent to the `error` port
    pub stderr: Vec<u8>,

    /// Request to exit with the given error code
    pub exit: Option<i32>,
}

impl Output<'_> {
    /// Prints `stdout` and `stderr` to the console
    pub fn print(&self) -> std::io::Result<()> {
        if !self.stdout.is_empty() {
            let mut stdout = std::io::stdout().lock();
            stdout.write_all(&self.stdout)?;
            stdout.flush()?;
        }
        if !self.stderr.is_empty() {
            let mut stderr = std::io::stderr().lock();
            stderr.write_all(&self.stderr)?;
            stderr.flush()?;
        }
        Ok(())
    }

    /// Checks the results
    ///
    /// `stdout` and `stderr` are printed, and `exit(..)` is called if it has
    /// been requested by the VM.
    pub fn check(&self) -> std::io::Result<()> {
        self.print()?;
        if let Some(e) = self.exit {
            log::info!("requested exit ({e})");

            #[cfg(not(target_arch = "wasm32"))]
            std::process::exit(e);

            #[cfg(target_arch = "wasm32")]
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "exit requested",
            ));
        }
        Ok(())
    }
}

/// Handle to the Varvara system
pub struct Varvara {
    system: system::System,
    console: console::Console,
    datetime: datetime::Datetime,
    audio: audio::Audio,
    screen: screen::Screen,
    mouse: mouse::Mouse,
    file: file::File,
    controller: controller::Controller,

    /// Flags indicating if we've already printed a warning about a missing dev
    already_warned: [bool; 16],
}

impl Default for Varvara {
    fn default() -> Self {
        Self::new()
    }
}

impl Device for Varvara {
    fn deo(&mut self, vm: &mut Uxn, target: u8) -> bool {
        match target & 0xF0 {
            system::SystemPorts::BASE => self.system.deo(vm, target),
            console::ConsolePorts::BASE => self.console.deo(vm, target),
            datetime::DatetimePorts::BASE => self.datetime.deo(vm, target),
            screen::ScreenPorts::BASE => self.screen.deo(vm, target),
            mouse::MousePorts::BASE => self.mouse.set_active(),
            f if file::FilePorts::matches(f) => self.file.deo(vm, target),
            controller::ControllerPorts::BASE => (),
            a if audio::AudioPorts::matches(a) => self.audio.deo(vm, target),

            // Default case
            t => self.warn_missing(t),
        }
        !self.system.should_exit()
    }
    fn dei(&mut self, vm: &mut Uxn, target: u8) {
        match target & 0xF0 {
            system::SystemPorts::BASE => self.system.dei(vm, target),
            console::ConsolePorts::BASE => self.console.dei(vm, target),
            datetime::DatetimePorts::BASE => self.datetime.dei(vm, target),
            screen::ScreenPorts::BASE => self.screen.dei(vm, target),
            mouse::MousePorts::BASE => self.mouse.set_active(),
            f if file::FilePorts::matches(f) => (),
            controller::ControllerPorts::BASE => (),
            a if audio::AudioPorts::matches(a) => self.audio.dei(vm, target),

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
            system: system::System::new(),
            datetime: datetime::Datetime,
            audio: audio::Audio::new(),
            screen: screen::Screen::new(),
            mouse: mouse::Mouse::new(),
            file: file::File::new(),
            controller: controller::Controller::new(),

            already_warned: [false; 16],
        }
    }

    /// Resets the CPU, loading extra data into expansion memory
    ///
    /// Note that the audio stream handles are unchanged, so any audio worker
    /// threads can continue to run.
    pub fn reset(&mut self, extra: &[u8]) {
        self.system.reset(extra);
        self.console = console::Console::new();
        self.audio.reset();
        self.screen = screen::Screen::new();
        self.mouse = mouse::Mouse::new();
        self.file = file::File::new();
        self.controller = controller::Controller::new();
        self.already_warned.fill(false);
    }

    /// Checks whether the SHIFT key is currently down
    fn warn_missing(&mut self, t: u8) {
        if !self.already_warned[usize::from(t >> 4)] {
            warn!("unimplemented device {t:#02x}");
            self.already_warned[usize::from(t >> 4)] = true;
        }
    }

    /// Calls the screen vector
    ///
    /// This function must be called at 60 Hz
    pub fn redraw(&mut self, vm: &mut Uxn) {
        let e = self.screen.update(vm);
        self.process_event(vm, e);
    }

    /// Returns the current output state of the system
    ///
    /// This is not idempotent; the output is taken from various accumulators
    /// and will be empty if this is called multiple times.
    #[must_use]
    pub fn output(&mut self, vm: &Uxn) -> Output {
        Output {
            size: self.screen.size(),
            frame: self.screen.frame(vm),
            hide_mouse: self.mouse.active(),
            stdout: self.console.stdout(),
            stderr: self.console.stderr(),
            exit: self.system.exit(),
        }
    }

    /// Sends arguments to the console device
    ///
    /// Leaves the console type set to `stdin`, and returns the current output
    /// state of the system
    pub fn send_args(&mut self, vm: &mut Uxn, args: &[String]) -> Output {
        for (i, a) in args.iter().enumerate() {
            self.console.set_type(vm, console::Type::Argument);
            for c in a.bytes() {
                self.process_event(vm, self.console.update(vm, c));
            }

            let ty = if i == args.len() - 1 {
                console::Type::ArgumentEnd
            } else {
                console::Type::ArgumentSpacer
            };
            self.console.set_type(vm, ty);
            self.process_event(vm, self.console.update(vm, b'\n'));
        }
        self.console.set_type(vm, console::Type::Stdin);
        self.output(vm)
    }

    /// Send a character from the keyboard (controller) device
    pub fn char(&mut self, vm: &mut Uxn, k: u8) {
        let e = self.controller.char(vm, k);
        self.process_event(vm, e);
    }

    /// Press a key on the controller device
    pub fn pressed(&mut self, vm: &mut Uxn, k: Key, repeat: bool) {
        if let Some(e) = self.controller.pressed(vm, k, repeat) {
            self.process_event(vm, e);
        }
    }

    /// Release a key on the controller device
    pub fn released(&mut self, vm: &mut Uxn, k: Key) {
        if let Some(e) = self.controller.released(vm, k) {
            self.process_event(vm, e);
        }
    }

    /// Send a character from the console device
    pub fn console(&mut self, vm: &mut Uxn, c: u8) {
        let e = self.console.update(vm, c);
        self.process_event(vm, e);
    }

    /// Updates the mouse state
    pub fn mouse(&mut self, vm: &mut Uxn, m: MouseState) {
        if let Some(e) = self.mouse.update(vm, m) {
            self.process_event(vm, e);
        }
    }

    /// Processes pending audio events
    pub fn audio(&mut self, vm: &mut Uxn) {
        for i in 0..audio::DEV_COUNT {
            if let Some(e) = self.audio.update(vm, usize::from(i)) {
                self.process_event(vm, e);
            }
        }
    }

    /// Processes a single vector event
    ///
    /// Events with an unassigned vector (i.e. 0) are ignored
    fn process_event(&mut self, vm: &mut Uxn, e: Event) {
        if e.vector != 0 {
            if let Some(d) = e.data {
                vm.write_dev_mem(d.addr, d.value);
            }
            vm.run(self, e.vector);
            if let Some(d) = e.data {
                if d.clear {
                    vm.write_dev_mem(d.addr, 0);
                }
            }
        }
    }

    /// Returns the set of audio stream data handles
    pub fn audio_streams(&self) -> [Arc<Mutex<audio::StreamData>>; 4] {
        [0, 1, 2, 3].map(|i| self.audio.stream(i))
    }

    /// Sets the global mute flag for audio
    pub fn audio_set_muted(&mut self, m: bool) {
        self.audio.set_muted(m)
    }
}
