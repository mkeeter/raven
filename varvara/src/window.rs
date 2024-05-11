use crate::{mouse::Mouse, screen::Screen, Event};
use minifb::{MouseMode, Scale, Window as FbWindow, WindowOptions};
use std::sync::mpsc;
use uxn::Uxn;

pub struct Window {
    pub screen: Screen,
    pub mouse: Mouse,

    has_mouse: bool,
    window: FbWindow,
}

const APP_NAME: &str = "Varvara";
impl Window {
    pub fn new(tx: mpsc::Sender<Event>) -> Self {
        const WIDTH: u16 = 640;
        const HEIGHT: u16 = 360;
        let screen = Screen::new(WIDTH, HEIGHT);
        let mouse = Mouse::new();

        let window = FbWindow::new(
            APP_NAME,
            WIDTH as usize,
            HEIGHT as usize,
            WindowOptions::default(),
        )
        .unwrap();

        std::thread::spawn(move || loop {
            if tx.send(Event::Window).is_err() {
                return;
            }
            std::thread::sleep(std::time::Duration::from_micros(16600));
        });
        Self {
            screen,
            mouse,

            has_mouse: false,
            window,
        }
    }

    pub fn set_mouse(&mut self) {
        if !self.has_mouse {
            self.has_mouse = true;
            self.window.set_cursor_visibility(false);
        }
    }

    pub fn event(&mut self, vm: &mut Uxn) -> impl Iterator<Item = u16> {
        // The screen vector should always be called
        let v = self.screen.event(vm);

        // The mouse vector should be called if it changed
        let m = if self.has_mouse {
            let mouse_pos =
                self.window.get_mouse_pos(MouseMode::Clamp).unwrap();
            let mouse_scroll = self.window.get_scroll_wheel();
            self.mouse.event(vm, mouse_pos, mouse_scroll, 0)
        } else {
            None
        };
        [Some(v), m].into_iter().flatten()
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
        self.window = FbWindow::new(
            APP_NAME,
            width as usize,
            height as usize,
            WindowOptions {
                scale: Scale::X2,
                ..WindowOptions::default()
            },
        )
        .unwrap();
        if self.has_mouse {
            self.window.set_cursor_visibility(false);
        }
    }
}
