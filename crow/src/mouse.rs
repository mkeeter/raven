use raven::{Ports, Uxn};
use zerocopy::{AsBytes, BigEndian, FromBytes, FromZeroes, U16};

#[derive(AsBytes, FromZeroes, FromBytes)]
#[repr(C)]
pub struct MousePorts {
    vector: U16<BigEndian>,
    x: U16<BigEndian>,
    y: U16<BigEndian>,
    state: u8,
    _padding1: u8,
    _padding2: u16,
    scroll_x: U16<BigEndian>,
    scroll_y: U16<BigEndian>,
    _padding3: u16,
}

impl Ports for MousePorts {
    const BASE: u8 = 0x90;
}

#[derive(Default)]
pub struct Mouse {
    pos: (f32, f32),
    scroll_x: f32,
    scroll_y: f32,
    buttons: u8,
}

impl Mouse {
    pub fn new() -> Self {
        Mouse::default()
    }

    /// Updates the internal mouse state, return the vector if state has changed
    pub fn update(
        &mut self,
        vm: &mut Uxn,
        pos: (f32, f32),
        scroll: Option<(f32, f32)>,
        buttons: u8,
    ) -> Option<u16> {
        let mut changed = false;
        let m = vm.dev_mut::<MousePorts>();

        if pos != self.pos {
            m.x.set(pos.0 as u16);
            m.y.set(pos.1 as u16);
            changed = true;
            self.pos = pos;
        }

        if let Some((sx, sy)) = scroll {
            self.scroll_x += sx;
            self.scroll_y += sy;
        }

        // Send scrolls as one-tick updates on a per-frame basis
        if self.scroll_x > 1.0 {
            changed = true;
            m.scroll_x.set(1);
            self.scroll_x -= 1.0;
        } else if self.scroll_x < -1.0 {
            changed = true;
            m.scroll_x.set(0xFFFF);
            self.scroll_x += 1.0;
        }
        if self.scroll_y > 1.0 {
            changed = true;
            m.scroll_y.set(1);
            self.scroll_y -= 1.0;
        } else if self.scroll_y < -1.0 {
            changed = true;
            m.scroll_y.set(0xFFFF);
            self.scroll_y += 1.0;
        }

        if buttons != self.buttons {
            m.state = buttons;
            changed = true;
            self.buttons = buttons;
        }

        if changed {
            Some(m.vector.get())
        } else {
            None
        }
    }
}
