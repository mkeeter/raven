use crate::Event;
use cpal::traits::StreamTrait;
use std::{
    collections::VecDeque,
    mem::offset_of,
    sync::{Arc, Mutex},
};
use uxn::{Ports, Uxn};
use zerocopy::{AsBytes, BigEndian, FromBytes, FromZeroes, U16};

#[derive(AsBytes, FromZeroes, FromBytes)]
#[repr(C)]
pub struct AudioPorts {
    vector: U16<BigEndian>,
    position: U16<BigEndian>,
    output: u8,
    duration: U16<BigEndian>,
    _padding: u8,
    adsr: Envelope,
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

    fn duration(&self) -> f32 {
        // No idea what's going on here; copied from the reference impl
        let dur = self.duration.get();
        if dur > 0 {
            dur as f32
        } else {
            let len = self.length.get();
            let pitch = self.pitch.note();
            let scale = TUNING[pitch as usize] / TUNING[0x28];
            len as f32 / (scale * 44.1)
        }
    }
}

const SAMPLE_RATE: u32 = 44100;

/// Decoder for the `adsr` port
#[derive(Copy, Clone, Default, AsBytes, FromZeroes, FromBytes)]
#[repr(C)]
struct Envelope(U16<BigEndian>);
impl Envelope {
    fn attack(&self) -> Option<f32> {
        let a = (self.0.get() >> 12) as u8 & 0xF;
        if a == 0 {
            None
        } else {
            let a = a as f32 * 64.0;
            Some(1000.0 / (a * SAMPLE_RATE as f32))
        }
    }
    fn decay(&self) -> f32 {
        let d = (((self.0.get() >> 8) as u8 & 0xF) as f32 * 64.0).max(10.0);
        1000.0 / (d * SAMPLE_RATE as f32)
    }
    fn sustain(&self) -> f32 {
        ((self.0.get() >> 4) as u8 & 0xF) as f32 / 255.0
    }
    fn release(&self) -> f32 {
        let r = (self.0.get() as u8 & 0xF) as f32 * 64.0;
        1000.0 / (r * SAMPLE_RATE as f32)
    }
    fn disabled(&self) -> bool {
        self.0.get() == 0
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

/// Decoder for the `pitch` port
#[derive(Copy, Clone, AsBytes, FromZeroes, FromBytes)]
#[repr(C)]
struct Pitch(u8);
impl Pitch {
    fn loop_sample(&self) -> bool {
        (self.0 >> 7) == 0
    }
    fn note(&self) -> u8 {
        (self.0 & 0x7F).max(20) - 20
    }
    fn is_empty(&self) -> bool {
        self.0 == 0
    }
}

// From `audio.c` in the original implementation
#[allow(clippy::excessive_precision)]
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

const MIDDLE_C: f32 = 261.6;

struct Stream {
    stream: cpal::Stream,
    data: Arc<Mutex<StreamData>>,
}

#[derive(Debug)]
enum Stage {
    Attack(f32),
    Decay,
    Sustain,
    Release,
}

struct StreamData {
    samples: Vec<u8>,
    loop_sample: bool,

    playing: bool,

    /// Position within the sample array, as a fraction
    pos: f32,

    /// Amount to increment `pos` on each sample
    inc: f32,

    /// Current stage
    stage: Stage,

    /// Current volume, modified by the envelope
    vol: f32,

    /// Envelope
    envelope: Envelope,

    /// Remaining time
    duration: f32,

    /// Set in the audio thread when the note is done
    done: bool,

    index: usize,
}

impl Default for StreamData {
    fn default() -> Self {
        Self {
            index: 0,
            samples: vec![],
            loop_sample: false,
            playing: false,
            pos: 0.0,
            inc: 0.0,
            duration: 0.0,
            stage: Stage::Attack(0.0),
            vol: 0.0,
            envelope: Envelope(0.into()),
            done: false,
        }
    }
}

impl StreamData {
    fn next(&mut self, data: &mut [f32], _opt: &cpal::OutputCallbackInfo) {
        if self.duration <= 0.0 {
            self.done = true;
        }
        self.duration -= (data.len() / 2) as f32 / SAMPLE_RATE as f32 * 1000.0;
        if self.playing {
            let mut i = 0;

            while i < data.len() {
                let wrap = self.samples.len() as f32;
                if self.pos >= wrap {
                    if self.loop_sample {
                        self.pos %= wrap;
                    } else {
                        self.playing = false;
                        break;
                    }
                }

                let lo = self.samples[self.pos.floor() as usize] as f32;
                let hi = self.samples[(self.pos.ceil() % wrap) as usize] as f32;
                let frac = self.pos % 1.0;

                let mut d = hi * frac + lo * (1.0 - frac);
                d *= self.vol;
                d = (d).min(u8::MAX as f32);
                d -= 128.0;
                d /= 512.0; // scale to ±0.5

                data[i] = d;
                data[i + 1] = d;
                i += 2;

                self.pos += self.inc;
                match self.stage {
                    Stage::Attack(a) => {
                        self.vol += a;
                        if self.vol >= 1.0 {
                            self.stage = Stage::Decay;
                            self.vol = 1.0;
                        }
                    }
                    Stage::Decay => {
                        self.vol -= self.envelope.decay();
                        if self.vol < 0.0 || self.vol <= self.envelope.sustain()
                        {
                            self.stage = Stage::Sustain;
                            self.vol = self.envelope.sustain();
                        }
                    }
                    Stage::Sustain => {
                        self.vol = self.envelope.sustain();
                    }
                    Stage::Release => {
                        self.vol = if self.vol <= 0.0
                            || self.envelope.release() <= 0.0
                        {
                            0.0
                        } else {
                            self.vol - self.envelope.release()
                        };
                    }
                }
            }
        } else {
            data.fill(0.0);
        }
    }
}

pub struct Audio {
    device: cpal::Device,
    config: cpal::StreamConfig,
    streams: [Stream; 4],
}

impl Audio {
    pub fn new() -> Self {
        use cpal::traits::{DeviceTrait, HostTrait};
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .expect("no output device available");
        let mut supported_configs_range = device
            .supported_output_configs()
            .expect("error while querying configs");

        let supported_config = supported_configs_range
            .find_map(|c| c.try_with_sample_rate(cpal::SampleRate(SAMPLE_RATE)))
            .expect("no supported config?");
        let config = supported_config.config();

        let stream_data =
            [(); 4].map(|_| Arc::new(Mutex::new(StreamData::default())));
        let streams = [0, 1, 2, 3].map(|i| {
            let d = stream_data[i].clone();
            let stream = device
                .build_output_stream(
                    &config,
                    move |data: &mut [f32], opt: &cpal::OutputCallbackInfo| {
                        d.lock().unwrap().next(data, opt);
                    },
                    move |err| {
                        panic!("{err}");
                    },
                    None,
                )
                .expect("could not build stream");
            stream.play().unwrap();
            Stream {
                stream,
                data: stream_data[i].clone(),
            }
        });

        Audio {
            device,
            config,
            streams,
        }
    }

    /// Push any relevant "note done" vectors to the event queue
    pub fn update(&self, vm: &Uxn, queue: &mut VecDeque<Event>) {
        for (i, s) in self.streams.iter().enumerate() {
            let mut s = s.data.lock().unwrap();
            if std::mem::take(&mut s.done) {
                let p = vm.dev_i::<AudioPorts>(i);
                let vector = p.vector.get();
                if vector != 0 {
                    queue.push_back(Event { data: None, vector });
                }
            }
        }
    }

    pub fn deo(&mut self, vm: &mut Uxn, target: u8) {
        let i = (target - AudioPorts::BASE) as usize / 0x10;
        if target == AudioPorts::PITCH + i as u8 * 16 {
            let p = vm.dev_i::<AudioPorts>(i);
            if p.pitch.is_empty() {
                let mut d = self.streams[i].data.lock().unwrap();
                d.stage = Stage::Release;
                d.duration = p.duration();
            } else {
                // No idea what's going on here!
                let len = p.length.get();
                let sample_rate = if len <= 256 {
                    len as f32
                } else {
                    SAMPLE_RATE as f32 / MIDDLE_C
                };

                let mut d = self.streams[i].data.lock().unwrap();

                // Copy the entire sample into RAM, reusing allocation
                let mut samples = std::mem::take(&mut d.samples);
                samples.clear();
                let base_addr = p.addr.get();
                for i in 0..len {
                    samples.push(vm.ram_read_byte(base_addr + i));
                }
                let inc = TUNING[p.pitch.note() as usize] * sample_rate;
                let attack = p.adsr.attack();

                let duration = p.duration();

                *d = StreamData {
                    index: i,
                    samples,
                    loop_sample: p.pitch.loop_sample(),
                    pos: 0.0,
                    inc,
                    duration,
                    done: false,

                    vol: if p.adsr.disabled() || attack.is_some() {
                        0.0
                    } else {
                        1.0
                    },
                    envelope: p.adsr,
                    stage: if let Some(a) = attack {
                        Stage::Attack(a)
                    } else {
                        Stage::Decay
                    },
                    playing: true,
                };
            }
        }
    }
    pub fn dei(&mut self, vm: &mut Uxn, target: u8) {
        let stream = (target - AudioPorts::BASE) as usize / 0x10;
        match target {
            AudioPorts::PITCH => panic!(),
            _ => (),
        }
    }
}
