use crate::Event;
use minifb::{Scale, Window, WindowOptions};
use std::sync::mpsc;
use uxn::{Device, Ports, Uxn};
use zerocopy::{AsBytes, BigEndian, FromBytes, FromZeroes, U16};

#[derive(AsBytes, FromZeroes, FromBytes)]
#[repr(C)]
pub struct ScreenPorts {
    vector: U16<BigEndian>,
    width: U16<BigEndian>,
    height: U16<BigEndian>,
    auto: Auto,
    _padding: u8,
    x: U16<BigEndian>,
    y: U16<BigEndian>,
    addr: U16<BigEndian>,
    pixel: Pixel,
    sprite: Sprite,
}

impl Ports for ScreenPorts {
    const BASE: u8 = 0x20;
}

impl ScreenPorts {
    // To ensure proper ordering, the 'read from device' operation (DEO) happens
    // when the first byte is touched; the 'write to device' (DEI) operation
    // happens when the second byte is touched.
    const WIDTH_R: u8 = Self::BASE | std::mem::offset_of!(Self, width) as u8;
    const WIDTH_W: u8 = Self::WIDTH_R + 1;
    const HEIGHT_R: u8 = Self::BASE | std::mem::offset_of!(Self, height) as u8;
    const HEIGHT_W: u8 = Self::HEIGHT_R + 1;
    const PIXEL: u8 = Self::BASE | std::mem::offset_of!(Self, pixel) as u8;
    const SPRITE: u8 = Self::BASE | std::mem::offset_of!(Self, sprite) as u8;
}

pub struct Screen {
    buffer: Vec<u32>,
    foreground: Vec<u8>,
    background: Vec<u8>,
    window: Window,
    width: u16,
    height: u16,
}

enum Layer {
    Foreground,
    Background,
}

/// Decoder for the `pixel` port
#[derive(Copy, Clone, AsBytes, FromZeroes, FromBytes)]
#[repr(C)]
struct Pixel(u8);

impl Pixel {
    fn color(&self) -> u8 {
        self.0 & 0b11
    }
    fn fill(&self) -> bool {
        (self.0 & (1 << 7)) != 0
    }
    fn layer(&self) -> Layer {
        if (self.0 & (1 << 6)) != 0 {
            Layer::Foreground
        } else {
            Layer::Background
        }
    }
    fn flip_y(&self) -> bool {
        (self.0 & (1 << 5)) != 0
    }
    fn flip_x(&self) -> bool {
        (self.0 & (1 << 4)) != 0
    }
}

/// Decoder for the `sprite` port
#[derive(Copy, Clone, AsBytes, FromZeroes, FromBytes)]
#[repr(C)]
struct Sprite(u8);
impl Sprite {
    fn color(&self) -> u8 {
        self.0 & 0b1111
    }
    fn two_bpp(&self) -> bool {
        (self.0 & (1 << 7)) != 0
    }
    fn layer(&self) -> Layer {
        if (self.0 & (1 << 6)) != 0 {
            Layer::Foreground
        } else {
            Layer::Background
        }
    }
    fn flip_y(&self) -> bool {
        (self.0 & (1 << 5)) != 0
    }
    fn flip_x(&self) -> bool {
        (self.0 & (1 << 4)) != 0
    }
}

/// Decoder for the `auto` port
#[derive(Copy, Clone, AsBytes, FromZeroes, FromBytes)]
#[repr(C)]
struct Auto(u8);
impl Auto {
    fn len(&self) -> u8 {
        self.0 >> 4
    }
    fn addr(&self) -> bool {
        (self.0 & (1 << 2)) != 0
    }
    fn y(&self) -> bool {
        (self.0 & (1 << 1)) != 0
    }
    fn x(&self) -> bool {
        (self.0 & (1 << 0)) != 0
    }
}

const APP_NAME: &str = "Varvara";

impl Screen {
    pub fn new(tx: mpsc::Sender<Event>) -> Self {
        const WIDTH: u16 = 640;
        const HEIGHT: u16 = 360;
        const SIZE: usize = WIDTH as usize * HEIGHT as usize;
        let buffer: Vec<u32> = vec![0; SIZE];
        let foreground: Vec<u8> = vec![0; SIZE];
        let background: Vec<u8> = vec![0; SIZE];

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
        Self {
            buffer,
            foreground,
            background,
            window,
            width: WIDTH,
            height: HEIGHT,
        }
    }

    pub fn event(&mut self, vm: &mut Uxn) -> u16 {
        // Nothing to do here, but return the screen vector
        vm.dev::<ScreenPorts>().vector.get()
    }

    /// Redraws the window and handles miscellaneous polling
    ///
    /// Returns `true` if the window is still open; `false` otherwise
    pub fn update(&mut self, vm: &Uxn) -> bool {
        self.buffer.resize(self.foreground.len(), 0u32);
        let sys = vm.dev::<crate::system::SystemPorts>();
        let colors = [0, 1, 2, 3].map(|i| sys.color(i));
        for ((&f, &b), o) in self
            .foreground
            .iter()
            .zip(&self.background)
            .zip(self.buffer.iter_mut())
        {
            let i = if f != 0 { f } else { b };
            *o = colors[i as usize];
        }
        self.window
            .update_with_buffer(
                &self.buffer,
                self.width as usize,
                self.height as usize,
            )
            .unwrap();
        self.window.is_open()
    }

    fn reopen(&mut self) {
        self.window = Window::new(
            APP_NAME,
            self.width as usize,
            self.height as usize,
            WindowOptions {
                scale: Scale::X2,
                ..WindowOptions::default()
            },
        )
        .unwrap();
        let size = self.width as usize * self.height as usize;
        self.foreground.resize(size, 0u8);
        self.background.resize(size, 0u8);
        self.buffer.resize(size, 0u32);
    }

    fn set_pixel(&mut self, layer: Layer, x: u16, y: u16, color: u8) {
        if x >= self.width || y >= self.height {
            return;
        }
        let i = x as usize + y as usize * self.width as usize;
        let pixels = match layer {
            Layer::Foreground => &mut self.foreground,
            Layer::Background => &mut self.background,
        };
        pixels[i] = color;
    }

    /// Executes the `pixel` operation
    fn pixel(&mut self, vm: &mut Uxn) {
        let v = vm.dev::<ScreenPorts>();
        let p = v.pixel;
        let auto = v.auto;

        let x = v.x.get();
        let y = v.y.get();

        if p.fill() {
            let xr = if p.flip_x() { 0..x } else { x..self.width };
            let yr = if p.flip_y() { 0..y } else { y..self.height };
            for x in xr {
                for y in yr.clone() {
                    self.set_pixel(p.layer(), x, y, p.color());
                }
            }
        } else {
            self.set_pixel(p.layer(), x, y, p.color());
            let v = vm.dev_mut::<ScreenPorts>();
            if auto.x() {
                v.x.set(v.x.get().wrapping_add(1));
            }
            if auto.y() {
                v.y.set(v.y.get().wrapping_add(1));
            }
        }
    }

    fn sprite(&mut self, vm: &mut Uxn) {
        let v = vm.dev::<ScreenPorts>();
        let s = v.sprite;

        const BLENDING: [[u8; 16]; 4] = [
            [0, 0, 0, 0, 1, 0, 1, 1, 2, 2, 0, 2, 3, 3, 3, 0],
            [0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3],
            [1, 2, 3, 1, 1, 2, 3, 1, 1, 2, 3, 1, 1, 2, 3, 1],
            [2, 3, 1, 2, 2, 3, 1, 2, 2, 3, 1, 2, 2, 3, 1, 2],
        ];
        const OPAQUE: [bool; 16] = [
            false, true, true, true, true, false, true, true, true, true,
            false, true, true, true, true, false,
        ];

        let auto = v.auto;

        // XXX THIS IS NOT A PLACE OF HONOR
        //
        // The exact behavior of the `sprite` port is emergent from the C code,
        // so this is written to match it when testing against the
        // `screen.blending.tal` example.
        let mut x = v.x.get();
        let mut y = v.y.get();
        for _n in 0..=auto.len() {
            let v = vm.dev::<ScreenPorts>();
            let mut addr = v.addr.get();

            for dy in 0..8 {
                let y = if s.flip_y() {
                    y.checked_add(7).and_then(|y| y.checked_sub(dy))
                } else {
                    y.checked_add(dy)
                };
                let Some(y) = y else {
                    continue;
                };
                if y >= self.height {
                    continue;
                }
                let lo = vm.ram_read(addr);
                let hi = if s.two_bpp() {
                    vm.ram_read(addr.wrapping_add(8))
                } else {
                    0
                };
                for dx in 0..8 {
                    let x = if s.flip_x() {
                        x.checked_add(7).and_then(|x| x.checked_sub(dx))
                    } else {
                        x.checked_add(dx)
                    };
                    let Some(x) = x else {
                        continue;
                    };
                    if x >= self.width {
                        continue;
                    }

                    let data = ((lo >> (7 - dx)) & 0b1)
                        | (((hi >> (7 - dx)) & 0b1) << 1);
                    if data != 0 || OPAQUE[s.color() as usize] {
                        let c = BLENDING[data as usize][s.color() as usize];
                        self.set_pixel(s.layer(), x, y, c);
                    }
                }
                addr = addr.wrapping_add(1);
            }
            // Skip the second byte if this is a 2bpp sprite
            if s.two_bpp() {
                addr = addr.wrapping_add(8);
            }
            // Update position within the loop
            if auto.y() {
                x = if s.flip_x() {
                    x.wrapping_sub(8)
                } else {
                    x.wrapping_add(8)
                };
            }
            if auto.x() {
                y = if s.flip_y() {
                    y.wrapping_sub(8)
                } else {
                    y.wrapping_add(8)
                };
            }
            // Update address globally
            if auto.addr() {
                let v = vm.dev_mut::<ScreenPorts>();
                v.addr.set(addr);
            }
        }
        let v = vm.dev_mut::<ScreenPorts>();
        if auto.x() {
            v.x.set(if s.flip_x() {
                v.x.get().wrapping_sub(8)
            } else {
                v.x.get().wrapping_add(8)
            })
        }
        if auto.y() {
            v.y.set(if s.flip_y() {
                v.y.get().wrapping_sub(8)
            } else {
                v.y.get().wrapping_add(8)
            })
        }
    }
}

impl Device for Screen {
    fn deo(&mut self, vm: &mut Uxn, target: u8) {
        let v = vm.dev::<ScreenPorts>();
        match target {
            ScreenPorts::WIDTH_W => {
                let new_width = v.width.get();
                if new_width != self.width {
                    self.width = new_width;
                    self.reopen();
                }
            }
            ScreenPorts::HEIGHT_W => {
                let new_height = v.height.get();
                if new_height != self.height {
                    self.height = new_height;
                    self.reopen();
                }
            }
            ScreenPorts::PIXEL => {
                self.pixel(vm);
            }
            ScreenPorts::SPRITE => {
                self.sprite(vm);
            }
            _ => (),
        }
        // Nothing to do here (yet)
    }
    fn dei(&mut self, vm: &mut Uxn, target: u8) {
        let v = vm.dev_mut::<ScreenPorts>();
        match target {
            ScreenPorts::WIDTH_R => {
                v.width.set(self.width);
            }
            ScreenPorts::HEIGHT_R => {
                v.height.set(self.height);
            }
            _ => (),
        }
    }
}
