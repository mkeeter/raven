use crate::{screen::Screen, Event};
use minifb::{Scale, Window, WindowOptions};
use std::sync::mpsc;
use uxn::Uxn;

pub struct Gui {
    pub screen: Screen,
    window: Window,
}

const APP_NAME: &str = "Varvara";
impl Gui {
    pub fn new(tx: mpsc::Sender<Event>) -> Self {
        const WIDTH: u16 = 640;
        const HEIGHT: u16 = 360;
        let screen = Screen::new(WIDTH, HEIGHT);

        let mut window = Window::new(
            APP_NAME,
            WIDTH as usize,
            HEIGHT as usize,
            WindowOptions::default(),
        )
        .unwrap();
        window.limit_update_rate(Some(std::time::Duration::from_micros(16600)));

        std::thread::spawn(move || loop {
            if tx.send(Event::Screen).is_err() {
                return;
            }
            std::thread::sleep(std::time::Duration::from_micros(16600));
        });
        Self { screen, window }
    }

    /// Redraws the window and handles miscellaneous polling
    ///
    /// Returns `true` if the window is still open; `false` otherwise
    pub fn update(&mut self, vm: &Uxn) -> bool {
        if self.screen.resized() {
            self.reopen();
        }
        let (buffer, width, height) = self.screen.update(vm);
        self.window
            .update_with_buffer(buffer, width as usize, height as usize)
            .unwrap();
        self.window.is_open()
    }

    /// Reopens the window, based on the screen size
    pub fn reopen(&mut self) {
        let (width, height) = self.screen.size();
        self.window = Window::new(
            APP_NAME,
            width as usize,
            height as usize,
            WindowOptions {
                scale: Scale::X2,
                ..WindowOptions::default()
            },
        )
        .unwrap();
    }
}
