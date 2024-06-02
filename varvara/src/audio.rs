use crate::Event;
use std::{collections::HashSet, mem::offset_of};
use uxn::{Ports, Uxn};
use zerocopy::{AsBytes, BigEndian, FromBytes, FromZeroes, U16};

#[derive(AsBytes, FromZeroes, FromBytes)]
#[repr(C)]
pub struct AudioPorts {
    vector: U16<BigEndian>,
    position: U16<BigEndian>,
    output: u8,
    _padding: [u8; 3],
    adsr: Adsr,
    length: U16<BigEndian>,
    addr: U16<BigEndian>,
    volume: Volume,
    pitch: Pitch,
}

impl Ports for AudioPorts {
    const BASE: u8 = 0x30;
}

impl AudioPorts {
    const PITCH: u8 = Self::BASE | offset_of!(Self, pitch) as u8;
}

/// Decoder for the `adsr` port
#[derive(Copy, Clone, AsBytes, FromZeroes, FromBytes)]
#[repr(C)]
struct Adsr(U16<BigEndian>);
impl Adsr {
    fn attack(&self) -> u8 {
        (self.0.get() >> 12) as u8 & 0xF
    }
    fn decay(&self) -> u8 {
        (self.0.get() >> 8) as u8 & 0xF
    }
    fn sustain(&self) -> u8 {
        (self.0.get() >> 4) as u8 & 0xF
    }
    fn release(&self) -> u8 {
        self.0.get() as u8 & 0xF
    }
}

/// Decoder for the `volume` port
#[derive(Copy, Clone, AsBytes, FromZeroes, FromBytes)]
#[repr(C)]
struct Volume(u8);
impl Volume {
    /// Returns the right-ear volume as a fraction between 0 and 1
    fn left(&self) -> f32 {
        ((self.0 >> 4) & 0xF) as f32 / 15.0
    }
    /// Returns the right-ear volume as a fraction between 0 and 1
    fn right(&self) -> f32 {
        (self.0 & 0xF) as f32 / 15.0
    }
}

/// Decoder for the `volume` port
#[derive(Copy, Clone, AsBytes, FromZeroes, FromBytes)]
#[repr(C)]
struct Pitch(u8);
impl Pitch {
    fn get_loop(&self) -> bool {
        (self.0 >> 7) != 0
    }
    fn note(&self) -> u8 {
        (self.0 & 0x7F).min(20)
    }
}

// From `audio.c` in the original implementation
const TUNING: [f32; 109] = [
    0.00058853, 0.00062352, 0.00066060, 0.00069988, 0.00074150, 0.00078559,
    0.00083230, 0.00088179, 0.00093423, 0.00098978, 0.00104863, 0.00111099,
    0.00117705, 0.00124704, 0.00132120, 0.00139976, 0.00148299, 0.00157118,
    0.00166460, 0.00176359, 0.00186845, 0.00197956, 0.00209727, 0.00222198,
    0.00235410, 0.00249409, 0.00264239, 0.00279952, 0.00296599, 0.00314235,
    0.00332921, 0.00352717, 0.00373691, 0.00395912, 0.00419454, 0.00444396,
    0.00470821, 0.00498817, 0.00528479, 0.00559904, 0.00593197, 0.00628471,
    0.00665841, 0.00705434, 0.00747382, 0.00791823, 0.00838908, 0.00888792,
    0.00941642, 0.00997635, 0.01056957, 0.01119807, 0.01186395, 0.01256941,
    0.01331683, 0.01410869, 0.01494763, 0.01583647, 0.01677815, 0.01777583,
    0.01883284, 0.01995270, 0.02113915, 0.02239615, 0.02372789, 0.02513882,
    0.02663366, 0.02821738, 0.02989527, 0.03167293, 0.03355631, 0.03555167,
    0.03766568, 0.03990540, 0.04227830, 0.04479229, 0.04745578, 0.05027765,
    0.05326731, 0.05643475, 0.05979054, 0.06334587, 0.06711261, 0.07110333,
    0.07533136, 0.07981079, 0.08455659, 0.08958459, 0.09491156, 0.10055530,
    0.10653463, 0.11286951, 0.11958108, 0.12669174, 0.13422522, 0.14220667,
    0.15066272, 0.15962159, 0.16911318, 0.17916918, 0.18982313, 0.20111060,
    0.21306926, 0.22573902, 0.23916216, 0.25338348, 0.26845044, 0.28441334,
    0.30132544,
];

pub struct Audio {
    device: cpal::Device,
    config: cpal::StreamConfig,
    stream: Option<cpal::Stream>,
}

impl Audio {
    pub fn new() -> Self {
        use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .expect("no output device available");
        let mut supported_configs_range = device
            .supported_output_configs()
            .expect("error while querying configs");
        let supported_config = supported_configs_range
            .next()
            .expect("no supported config?!")
            .with_max_sample_rate();
        let config = supported_config.config();

        let mut sample = 0;
        /*
        let stream = device
            .build_output_stream(
                &config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    for v in data.iter_mut() {
                        let t = sample as f32 / config.sample_rate.0 as f32;
                        *v = (t * 600.0).cos() * 1.0;
                        sample += 1;
                    }
                },
                move |err| {
                    panic!("{err}");
                },
                None,
            )
            .expect("could not build stream");
        stream.play().unwrap();
        */

        Audio {
            device,
            config,
            stream: None,
        }
    }

    pub fn deo(&mut self, vm: &mut Uxn, target: u8) {
        panic!()
    }
    pub fn dei(&mut self, vm: &mut Uxn, target: u8) {
        match target & 0x0F {
            AudioPorts::PITCH => panic!(),
            _ => (),
        }
    }
}
