use crate::{mouse::Mouse, screen::Screen};
use minifb::{
    MouseButton, MouseMode, Scale, Window as FbWindow, WindowOptions,
};
use uxn::Uxn;

pub struct Window {
    pub screen: Screen,
    pub mouse: Mouse,

    has_mouse: bool,
    window: FbWindow,
    frame: u64,
}

const APP_NAME: &str = "Varvara";
impl Window {
    pub fn new() -> Self {
        const WIDTH: u16 = 512;
        const HEIGHT: u16 = 320;
        let screen = Screen::new(WIDTH, HEIGHT);
        let mouse = Mouse::new();

        let mut window = FbWindow::new(
            APP_NAME,
            WIDTH as usize,
            HEIGHT as usize,
            WindowOptions {
                scale: Scale::X2,
                ..WindowOptions::default()
            },
        )
        .unwrap();
        window.set_target_fps(120);

        Self {
            screen,
            mouse,
            frame: 0,

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
        // The screen vector should be called every other frame, since we do
        // updates at ~120 FPS
        let v = if self.frame & 1 == 1 {
            Some(self.screen.event(vm))
        } else {
            None
        };
        self.frame = self.frame.wrapping_add(1);

        // The mouse vector should be called if it changed
        let m = if self.has_mouse {
            let mouse_pos =
                self.window.get_mouse_pos(MouseMode::Clamp).unwrap();
            let mouse_scroll = self.window.get_scroll_wheel();
            let buttons =
                [MouseButton::Left, MouseButton::Middle, MouseButton::Right]
                    .into_iter()
                    .enumerate()
                    .map(|(i, b)| (self.window.get_mouse_down(b) as u8) << i)
                    .fold(0, |a, b| a | b);
            self.mouse.event(vm, mouse_pos, mouse_scroll, buttons)
        } else {
            None
        };
        [v, m].into_iter().flatten()
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
        self.window.set_target_fps(120);
        if self.has_mouse {
            self.window.set_cursor_visibility(false);
        }
    }
}
