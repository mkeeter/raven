use crate::Event;
use uxn::{Ports, Uxn};
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

/// Stored mouse state
#[derive(Default)]
pub(crate) struct Mouse {
    /// Current position
    pos: (f32, f32),

    /// Accumulated scroll values, used for fractional scrolling
    scroll: (f32, f32),

    /// Bitfield of button state (bit 0: left, bit 1: middle, bit 2: right)
    buttons: u8,

    /// Set as true when a mouse DEI / DEO operator is called
    active: bool,
}

/// Update to mouse state
#[derive(Default, Debug)]
pub struct MouseState {
    /// Current position
    pub pos: (f32, f32),

    /// Accumulated scroll values, used for fractional scrolling
    pub scroll: (f32, f32),

    /// Bitfield of button state (bit 0: left, bit 1: middle, bit 2: right)
    pub buttons: u8,
}

impl Mouse {
    pub fn new() -> Self {
        Mouse::default()
    }

    /// Sets the active flag
    pub fn set_active(&mut self) {
        self.active = true
    }

    /// Checks whether the active flag has been set
    pub fn active(&self) -> bool {
        self.active
    }

    /// Updates the internal mouse state, pushing an event if it has changed
    pub fn update(
        &mut self,
        vm: &mut Uxn,
        state: MouseState,
        queue: &mut Vec<Event>,
    ) {
        let mut changed = false;
        let m = vm.dev_mut::<MousePorts>();

        if state.pos != self.pos {
            m.x.set(state.pos.0 as u16);
            m.y.set(state.pos.1 as u16);
            changed = true;
            self.pos = state.pos;
        }

        self.scroll.0 += state.scroll.0;
        self.scroll.1 += state.scroll.1;

        // Send scrolls as one-tick updates on a per-frame basis
        if self.scroll.0 > 1.0 {
            changed = true;
            m.scroll_x.set(1);
            self.scroll.0 -= 1.0;
        } else if self.scroll.0 < -1.0 {
            changed = true;
            m.scroll_x.set(0xFFFF);
            self.scroll.0 += 1.0;
        }
        if self.scroll.1 > 1.0 {
            changed = true;
            m.scroll_y.set(1);
            self.scroll.1 -= 1.0;
        } else if self.scroll.1 < -1.0 {
            changed = true;
            m.scroll_y.set(0xFFFF);
            self.scroll.1 += 1.0;
        }

        if state.buttons != self.buttons {
            m.state = state.buttons;
            changed = true;
            self.buttons = state.buttons;
        }

        if changed {
            queue.push(Event {
                data: None,
                vector: m.vector.get(),
            });
        }
    }
}
