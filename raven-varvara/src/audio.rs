use crate::Event;
use std::{
    collections::VecDeque,
    mem::offset_of,
    sync::atomic::{AtomicBool, Ordering},
    sync::{Arc, Mutex},
};
use uxn::{Ports, Uxn, DEV_SIZE};
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
    // Note that PITCH and POSITION are relative instead of absolute values,
    // because we have to support multiple Audio ports at different offsets.
    const PITCH: u8 = offset_of!(Self, pitch) as u8;
    const POSITION_H: u8 = offset_of!(Self, position) as u8;
    const POSITION_L: u8 = Self::POSITION_H + 1;
    const OUTPUT: u8 = offset_of!(Self, output) as u8;

    /// Checks whether the given value is in the audio ports memory space
    pub fn matches(t: u8) -> bool {
        (Self::BASE..Self::BASE + 0x10 * DEV_COUNT).contains(&t)
    }

    fn dev<'a>(vm: &'a Uxn, i: usize) -> &'a Self {
        let pos = Self::BASE + (i * DEV_SIZE) as u8;
        vm.dev_at(pos)
    }

    fn dev_mut<'a>(vm: &'a mut Uxn, i: usize) -> &'a mut Self {
        let pos = Self::BASE + (i * DEV_SIZE) as u8;
        vm.dev_mut_at(pos)
    }

    fn duration(&self) -> f32 {
        // No idea what's going on here; copied from the reference impl
        let dur = self.duration.get();
        if dur > 0 {
            dur as f32
        } else {
            let len = self.length.get();
            let pitch = self.pitch.note();
            let scale = TUNING[usize::from(pitch)] / TUNING[0x28];
            len as f32 / (scale * 44.1)
        }
    }
}

/// Number of audio devices
pub const DEV_COUNT: u8 = 4;

/// Expected audio sample rate
pub const SAMPLE_RATE: u32 = 44100;

/// Expected number of audio channels
#[cfg(not(target_arch = "wasm32"))]
pub const CHANNELS: usize = 2;

/// Expected number of audio channels (WebAssembly)
#[cfg(target_arch = "wasm32")]
pub const CHANNELS: usize = 1;

/// Number of samples to use for crossfade
const CROSSFADE_COUNT: usize = 200;

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
        ((self.0.get() >> 4) as u8 & 0xF) as f32 / 16.0
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
    done: Arc<AtomicBool>,
    data: Arc<Mutex<StreamData>>,
}

#[derive(Debug)]
enum Stage {
    Attack(f32),
    Decay,
    Sustain,
    Release,
}

/// Handle into an audio stream
pub struct StreamData {
    samples: Vec<u8>,
    loop_sample: bool,

    /// Computed samples from the previous stream, for crossfading
    crossfade: VecDeque<f32>,

    /// Position within the sample array, as a fraction
    pos: f32,

    /// Absolute position
    megapos: f32,

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

    /// Volume adjustment for left ear
    left: f32,

    /// Volume adjustment for right ear
    right: f32,

    /// Set in the audio thread when the note is done
    done: Arc<AtomicBool>,

    /// Flag to mute the audio stream from the GUI
    ///
    /// This is read-only in the [`StreamData`] and set by the parent
    muted: Arc<AtomicBool>,
}

impl StreamData {
    fn new(muted: Arc<AtomicBool>) -> Self {
        Self {
            samples: vec![],
            crossfade: VecDeque::new(),
            loop_sample: false,
            pos: 0.0,
            megapos: 0.0,
            inc: 0.0,
            duration: 0.0,
            stage: Stage::Attack(0.0),
            vol: 0.0,
            left: 0.0,
            right: 0.0,
            envelope: Envelope(0.into()),
            done: Arc::new(AtomicBool::new(false)),
            muted,
        }
    }

    /// Safely reads a sample, returning 0 if it's not valid
    fn get_sample(&self, f: usize) -> f32 {
        self.samples.get(f).cloned().unwrap_or(0) as f32
    }

    /// Fills the buffer with stream data
    pub fn next(&mut self, data: &mut [f32]) {
        self.duration -= (data.len() / 2) as f32 / SAMPLE_RATE as f32 * 1000.0;
        if self.duration <= 0.0 {
            self.done.store(true, Ordering::Relaxed);
        }
        let mut i = 0;
        let muted = self.muted.load(Ordering::Relaxed);

        while i < data.len() {
            let wrap = self.samples.len() as f32;
            let mut valid = true;
            if self.pos >= wrap {
                if self.loop_sample {
                    self.pos %= wrap;
                } else {
                    valid = false;
                }
            }

            let d = if valid {
                let lo = self.get_sample(self.pos.floor() as usize);
                let hi = self.get_sample((self.pos.ceil() % wrap) as usize);
                let frac = self.pos % 1.0;

                let mut d = hi * frac + lo * (1.0 - frac);
                d *= self.vol;
                d = (d).min(u8::MAX as f32);
                d -= 128.0;
                d /= 512.0; // scale to ±0.5
                d
            } else {
                0.0
            };

            static_assertions::const_assert!(CHANNELS == 1 || CHANNELS == 2);
            let d = if muted { 0.0 } else { d };
            match CHANNELS {
                1 => data[i] = d,
                2 => {
                    data[i] = d * self.left;
                    data[i + 1] = d * self.right;
                }
                _ => unreachable!(),
            };

            if !self.crossfade.is_empty() {
                let x = self.crossfade.len() as f32
                    / (CROSSFADE_COUNT as f32 - 1.0);
                for j in 0..CHANNELS {
                    let v = self.crossfade.pop_front().unwrap();
                    data[i + j] = v * x + data[i + j] * (1.0 - x);
                }
            }
            i += CHANNELS;

            self.pos += self.inc;
            self.megapos += self.inc;
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
                    if self.vol < 0.0 || self.vol <= self.envelope.sustain() {
                        self.stage = Stage::Sustain;
                        self.vol = self.envelope.sustain();
                    }
                }
                Stage::Sustain => {
                    self.vol = self.envelope.sustain();
                }
                Stage::Release => {
                    self.vol =
                        if self.vol <= 0.0 || self.envelope.release() <= 0.0 {
                            0.0
                        } else {
                            self.vol - self.envelope.release()
                        };
                }
            }
        }
    }
}

pub struct Audio {
    streams: [Stream; DEV_COUNT as usize],

    /// Flag to mute the audio stream from the GUI
    muted: Arc<AtomicBool>,
}

impl Audio {
    pub fn new() -> Self {
        let muted = Arc::new(AtomicBool::new(false));
        let stream_data = [(); 4]
            .map(|_| Arc::new(Mutex::new(StreamData::new(muted.clone()))));
        let streams = [0, 1, 2, 3].map(|i| Stream {
            done: stream_data[i].lock().unwrap().done.clone(),
            data: stream_data[i].clone(),
        });

        Audio { streams, muted }
    }

    /// Sets the global mute flag
    pub fn set_muted(&mut self, m: bool) {
        self.muted.store(m, Ordering::Relaxed);
    }

    /// Resets the audio stream data, preserving the same allocation
    pub fn reset(&mut self) {
        for s in &self.streams {
            *s.data.lock().unwrap() = StreamData::new(self.muted.clone());
            s.done.store(false, Ordering::Relaxed);
        }
    }

    /// Return the "note done" vector if the given channel is done
    pub fn update(&self, vm: &Uxn, i: usize) -> Option<Event> {
        if self.streams[i].done.swap(false, Ordering::Relaxed) {
            let p = AudioPorts::dev(vm, i);
            let vector = p.vector.get();
            Some(Event { data: None, vector })
        } else {
            None
        }
    }

    pub fn deo(&mut self, vm: &mut Uxn, target: u8) {
        let (i, target) = Self::decode_target(target);
        if target == AudioPorts::PITCH {
            let p = AudioPorts::dev(vm, i);
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

                // Compute a crossfade transition from the previous sample
                // (this may just be all zeros, which is fine)
                let mut d = self.streams[i].data.lock().unwrap();

                // Populate crossfade samples by sampling the previous stream
                let mut crossfade = std::mem::take(&mut d.crossfade);
                crossfade.resize(CROSSFADE_COUNT, 0.0f32);
                d.next(crossfade.make_contiguous());

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

                let done = self.streams[i].done.clone();
                done.store(false, Ordering::Relaxed);
                *d = StreamData {
                    samples,
                    crossfade,
                    loop_sample: p.pitch.loop_sample(),
                    pos: 0.0,
                    megapos: 0.0,
                    inc,
                    duration,
                    done,

                    vol: if p.adsr.disabled() || attack.is_some() {
                        0.0
                    } else {
                        1.0
                    },
                    left: p.volume.left(),
                    right: p.volume.right(),
                    envelope: p.adsr,
                    stage: if let Some(a) = attack {
                        Stage::Attack(a)
                    } else {
                        Stage::Decay
                    },
                    muted: self.muted.clone(),
                };
            }
        }
    }

    pub fn dei(&mut self, vm: &mut Uxn, target: u8) {
        let (i, target) = Self::decode_target(target);
        let p = AudioPorts::dev_mut(vm, i);

        // TODO: do we need the ability to read back DEI values without changing
        // the existing values in device port memory?
        match target {
            AudioPorts::POSITION_H => {
                let pos = self.streams[i].data.lock().unwrap().pos as u16;
                p.position = pos.into();
            }
            AudioPorts::POSITION_L => {
                // We assume POSITION_H is read first, so this is already loaded
            }
            AudioPorts::OUTPUT => {
                let vol = self.streams[i].data.lock().unwrap().vol * 255.0;
                p.output = vol as u8;
            }
            _ => (),
        }
    }

    /// Decodes a port address into an `(index, offset)` tuple
    fn decode_target(target: u8) -> (usize, u8) {
        let i = (target - AudioPorts::BASE) as usize / DEV_SIZE;
        (i, target & 0xF)
    }

    /// Returns a handle to the given stream data
    pub fn stream(&self, i: usize) -> Arc<Mutex<StreamData>> {
        self.streams[i].data.clone()
    }
}
