use crate::Event;
use std::mem::offset_of;
use uxn::{Ports, Uxn};
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
    const WIDTH_R: u8 = Self::BASE | offset_of!(Self, width) as u8;
    const WIDTH_W: u8 = Self::WIDTH_R + 1;
    const HEIGHT_R: u8 = Self::BASE | offset_of!(Self, height) as u8;
    const HEIGHT_W: u8 = Self::HEIGHT_R + 1;
    const PIXEL: u8 = Self::BASE | offset_of!(Self, pixel) as u8;
    const SPRITE: u8 = Self::BASE | offset_of!(Self, sprite) as u8;
}

#[derive(Copy, Clone, Default)]
struct ScreenPixel {
    fg: u8,
    bg: u8,
}

impl ScreenPixel {
    fn get(&self) -> u8 {
        if self.fg != 0 {
            self.fg
        } else {
            self.bg
        }
    }
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

pub struct Screen {
    /// Screen buffer
    pixels: Vec<ScreenPixel>,

    /// Local buffer for rendered RGBA values
    buffer: Vec<u8>,

    width: u16,
    height: u16,

    /// Flag indicating whether `buffer` should be recalculated
    changed: bool,

    /// Color palette
    colors: [u32; 4],
}

impl Default for Screen {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen {
    pub fn new() -> Self {
        const WIDTH: u16 = 512;
        const HEIGHT: u16 = 320;
        let size = WIDTH as usize * WIDTH as usize;
        let buffer = vec![0; size * 4];
        let pixels = vec![ScreenPixel::default(); size];
        Self {
            buffer,
            pixels,
            width: WIDTH,
            height: HEIGHT,
            changed: true,
            colors: [0; 4],
        }
    }

    /// Resizes our internal buffers to the new width and height
    fn resize(&mut self, width: u16, height: u16) {
        if width == self.width && height == self.height {
            return;
        }
        self.width = width;
        self.height = height;

        let size = self.width as usize * self.height as usize;
        self.pixels.resize(size, ScreenPixel::default());
        self.buffer.resize(size * 4, 0u8);
    }

    /// Returns the current size as a `(width, height)` tuple
    pub fn size(&self) -> (u16, u16) {
        (self.width, self.height)
    }

    /// Gets the current frame, returning a `(buffer, width, height)` tuple
    pub fn frame(&mut self, vm: &Uxn) -> &[u8] {
        let prev_colors = self.colors;
        let sys = vm.dev::<crate::system::SystemPorts>();
        self.colors = [0, 1, 2, 3].map(|i| sys.color(i));
        self.changed |= prev_colors != self.colors;

        if std::mem::take(&mut self.changed) {
            for (p, o) in self.pixels.iter().zip(self.buffer.chunks_mut(4)) {
                o.copy_from_slice(
                    &self.colors[(p.get() & 0b11) as usize].to_le_bytes(),
                );
            }
        }
        &self.buffer
    }

    fn set_pixel(&mut self, layer: Layer, x: u16, y: u16, color: u8) {
        if x >= self.width || y >= self.height {
            return;
        }
        let i = x as usize + y as usize * self.width as usize;
        // This should always be true, but we check to avoid a panic site
        if let Some(o) = self.pixels.get_mut(i) {
            match layer {
                Layer::Foreground => o.fg = color,
                Layer::Background => o.bg = color,
            };
        }
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
                let lo = vm.ram_read_byte(addr);
                let hi = if s.two_bpp() {
                    vm.ram_read_byte(addr.wrapping_add(8))
                } else {
                    0
                };
                addr = addr.wrapping_add(1);

                let y = y.wrapping_add(if s.flip_y() { 7 - dy } else { dy });
                if y >= self.height {
                    continue;
                }
                for dx in 0..8 {
                    let x =
                        x.wrapping_add(if s.flip_x() { 7 - dx } else { dx });
                    if x >= self.width {
                        continue;
                    }

                    let lo_bit = (lo >> (7 - dx)) & 0b1;
                    let hi_bit = (hi >> (7 - dx)) & 0b1; // 0 if !two_bpp
                    let data = (lo_bit | (hi_bit << 1)) as usize;
                    let color = s.color() as usize;
                    if data != 0 || OPAQUE[color] {
                        let c = BLENDING[data][color];
                        self.set_pixel(s.layer(), x, y, c);
                    }
                }
            }
            // Update position within the loop.  Note that we don't update the
            // ports here; they're updated outside the loop below.
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
            // Update the address port, skipping the second byte if this is a
            // 2bpp sprite (if not, addr is already incremented to the new
            // position, so just assign it)
            if auto.addr() {
                let v = vm.dev_mut::<ScreenPorts>();
                v.addr.set(if s.two_bpp() {
                    addr.wrapping_add(8)
                } else {
                    addr
                });
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

    /// Executes a DEO command against the screen
    pub fn deo(&mut self, vm: &mut Uxn, target: u8) {
        let v = vm.dev::<ScreenPorts>();
        self.changed = true;
        match target {
            ScreenPorts::WIDTH_W => {
                let new_width = v.width.get();
                self.resize(new_width, self.height);
            }
            ScreenPorts::HEIGHT_W => {
                let new_height = v.height.get();
                self.resize(self.width, new_height);
            }
            ScreenPorts::PIXEL => {
                self.pixel(vm);
            }
            ScreenPorts::SPRITE => {
                self.sprite(vm);
            }
            _ => (),
        }
    }

    /// Executes a DEI command against the screen
    pub fn dei(&mut self, vm: &mut Uxn, target: u8) {
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

    /// Called on screen update; returns the screen vector
    pub fn update(&mut self, vm: &mut Uxn) -> Event {
        // Nothing to do here, but return the screen vector
        let vector = vm.dev::<ScreenPorts>().vector.get();
        Event { data: None, vector }
    }
}
