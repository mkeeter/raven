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
    auto: u8,
    _padding: u8,
    x: U16<BigEndian>,
    y: U16<BigEndian>,
    addr: U16<BigEndian>,
    pixel: u8,
    sprite: u8,
}

impl Ports for ScreenPorts {
    const BASE: u8 = 0x20;
    fn assert_size() {
        static_assertions::assert_eq_size!(ScreenPorts, [u8; 16]);
    }
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
    width: usize,
    height: usize,
}

const APP_NAME: &str = "Varvara";

impl Screen {
    pub fn new(tx: mpsc::Sender<Event>) -> Self {
        const WIDTH: usize = 640;
        const HEIGHT: usize = 360;
        let buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];
        let foreground: Vec<u8> = vec![0; WIDTH * HEIGHT];
        let background: Vec<u8> = vec![0; WIDTH * HEIGHT];

        let mut window =
            Window::new(APP_NAME, WIDTH, HEIGHT, WindowOptions::default())
                .unwrap();
        window.limit_update_rate(Some(std::time::Duration::from_micros(16600)));

        std::thread::spawn(move || loop {
            if tx.send(Event::Screen).is_err() {
                return;
            }
            std::thread::sleep(std::time::Duration::from_micros(16666));
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
            .update_with_buffer(&self.buffer, self.width, self.height)
            .unwrap();
        self.window.is_open()
    }

    fn reopen(&mut self) {
        self.window = Window::new(
            APP_NAME,
            self.width,
            self.height,
            WindowOptions {
                scale: Scale::X2,
                ..WindowOptions::default()
            },
        )
        .unwrap();
        self.foreground = vec![0u8; self.width * self.height];
        self.background = vec![0u8; self.width * self.height];
        self.buffer = vec![0; self.width * self.height];
    }

    /// Executes the `pixel` operation
    fn pixel(&mut self, vm: &mut Uxn) {
        let v = vm.dev::<ScreenPorts>();
        let p = v.pixel;
        let color = p & 0b11;
        let fill = (p & (1 << 7)) != 0;
        let layer = (p & (1 << 6)) != 0;
        let flip_y = (p & (1 << 5)) != 0;
        let flip_x = (p & (1 << 4)) != 0;
        let auto = v.auto;

        let x = v.x.get() as usize;
        let y = v.y.get() as usize;
        let pixels = if layer {
            &mut self.foreground
        } else {
            &mut self.background
        };

        if fill {
            let xr = if flip_x { 0..x } else { x..self.width };
            let yr = if flip_y { 0..y } else { y..self.height };
            for x in xr {
                for y in yr.clone() {
                    let i = x + y * self.width;
                    if let Some(p) = pixels.get_mut(i) {
                        *p = color;
                    }
                }
            }
        } else if let Some(p) = pixels.get_mut(x + y * self.width) {
            *p = color;
            let auto_y = (auto & (1 << 1)) != 0;
            let auto_x = (auto & (1 << 0)) != 0;
            let v = vm.dev_mut::<ScreenPorts>();
            if auto_x {
                v.x.set(v.x.get().wrapping_add(1));
            }
            if auto_y {
                v.y.set(v.y.get().wrapping_add(1));
            }
        }
    }

    fn sprite(&mut self, vm: &mut Uxn) {
        let v = vm.dev::<ScreenPorts>();
        let p = v.sprite;

        let color = p & 0b1111;
        let two_bpp = (p & (1 << 7)) != 0;
        let layer = (p & (1 << 6)) != 0;
        let flip_y = (p & (1 << 5)) != 0;
        let flip_x = (p & (1 << 4)) != 0;

        let pixels = if layer {
            &mut self.foreground
        } else {
            &mut self.background
        };

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
        let auto_len = auto >> 4;
        let auto_addr = (auto & (1 << 2)) != 0;
        let auto_y = (auto & (1 << 1)) != 0;
        let auto_x = (auto & (1 << 0)) != 0;

        // XXX THIS IS NOT A PLACE OF HONOR
        //
        // The exact behavior of the `sprite` port is emergent from the C code,
        // so this is written to match it when testing against the
        // `screen.blending.tal` example.
        let mut x = v.x.get() as usize;
        let mut y = v.y.get() as usize;
        for _n in 0..=auto_len {
            let v = vm.dev::<ScreenPorts>();
            let mut addr = v.addr.get();

            for dy in 0..8 {
                let y = if flip_y {
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
                let hi = if two_bpp {
                    vm.ram_read(addr.wrapping_add(8))
                } else {
                    0
                };
                for dx in 0..8 {
                    let x = if flip_x {
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
                    let i = (x + y * self.width) % pixels.len();
                    if let Some(p) = pixels.get_mut(i) {
                        if data != 0 || OPAQUE[color as usize] {
                            *p = BLENDING[data as usize][color as usize];
                        }
                    }
                }
                addr = addr.wrapping_add(1);
            }
            // Skip the second byte if this is a 2bpp sprite
            if two_bpp {
                addr = addr.wrapping_add(8);
            }
            // Update position within the loop
            if auto_y {
                x = if flip_x {
                    x.wrapping_sub(8)
                } else {
                    x.wrapping_add(8)
                };
            }
            if auto_x {
                y = if flip_y {
                    y.wrapping_sub(8)
                } else {
                    y.wrapping_add(8)
                };
            }
            // Update address globally
            if auto_addr {
                let v = vm.dev_mut::<ScreenPorts>();
                v.addr.set(addr);
            }
        }
        let v = vm.dev_mut::<ScreenPorts>();
        if auto_x {
            v.x.set(if flip_x {
                v.x.get().wrapping_sub(8)
            } else {
                v.x.get().wrapping_add(8)
            })
        }
        if auto_y {
            v.y.set(if flip_y {
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
                let new_width = usize::from(v.width.get());
                if new_width != self.width {
                    self.width = new_width;
                    self.reopen();
                }
            }
            ScreenPorts::HEIGHT_W => {
                let new_height = usize::from(v.height.get());
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
                v.width.set(self.width as u16);
            }
            ScreenPorts::HEIGHT_R => {
                v.height.set(self.height as u16);
            }
            _ => (),
        }
    }
}
