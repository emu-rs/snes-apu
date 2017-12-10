use super::dsp::Dsp;
use super::super::apu::Apu;
use super::envelope::Envelope;
use super::brr_block_decoder::BrrBlockDecoder;
use super::dsp_helpers;
use super::gaussian::{HALF_KERNEL_SIZE, HALF_KERNEL};

const RESAMPLE_BUFFER_LEN: usize = 4;

#[derive(Clone, Copy)]
pub enum ResamplingMode {
    Linear,
    Gaussian,
}

#[derive(Clone, Copy)]
pub struct VoiceOutput {
    pub left_out: i32,
    pub right_out: i32,
    pub last_voice_out: i32,
}

impl VoiceOutput {
    pub fn default() -> VoiceOutput {
        VoiceOutput {
            left_out: 0,
            right_out: 0,
            last_voice_out: 0,
        }
    }
}

pub const VOICE_BUFFER_LEN: usize = 128;

pub struct VoiceBuffer {
    pub buffer: Box<[VoiceOutput]>,
    pub pos: i32,
}

impl VoiceBuffer {
    pub fn new() -> VoiceBuffer {
        VoiceBuffer {
            buffer: vec![VoiceOutput::default(); VOICE_BUFFER_LEN].into_boxed_slice(),
            pos: 0,
        }
    }

    pub fn write(&mut self, value: VoiceOutput) {
        self.buffer[self.pos as usize] = value;
        self.pos = (self.pos + 1) % (VOICE_BUFFER_LEN as i32);
    }
}

pub struct Voice {
    dsp: *mut Dsp,
    emulator: *mut Apu,

    pub envelope: Envelope,

    pub vol_left: u8,
    pub vol_right: u8,
    pub pitch_low: u8,
    pitch_high: u8,
    pub source: u8,
    pub pitch_mod: bool,
    pub noise_on: bool,
    pub echo_on: bool,

    sample_start_address: u32,
    loop_start_address: u32,
    brr_block_decoder: BrrBlockDecoder,
    sample_address: u32,
    sample_pos: i32,

    pub resampling_mode: ResamplingMode,
    resample_buffer: [i32; RESAMPLE_BUFFER_LEN],
    resample_buffer_pos: usize,

    pub output_buffer: VoiceBuffer,
    pub is_muted: bool,
    pub is_solod: bool,
}

impl Voice {
    pub fn new(dsp: *mut Dsp, emulator: *mut Apu, resampling_mode: ResamplingMode) -> Voice {
        Voice {
            dsp: dsp,
            emulator: emulator,

            envelope: Envelope::new(dsp),

            vol_left: 0,
            vol_right: 0,
            pitch_low: 0,
            pitch_high: 0x10,
            source: 0,
            pitch_mod: false,
            noise_on: false,
            echo_on: false,

            sample_start_address: 0,
            loop_start_address: 0,
            brr_block_decoder: BrrBlockDecoder::new(),
            sample_address: 0,
            sample_pos: 0,

            resampling_mode: resampling_mode,
            resample_buffer: [0; RESAMPLE_BUFFER_LEN],
            resample_buffer_pos: 0,

            output_buffer: VoiceBuffer::new(),
            is_muted: false,
            is_solod: false,
        }
    }

    #[inline]
    fn dsp(&self) -> &mut Dsp {
        unsafe {
            &mut (*self.dsp)
        }
    }

    #[inline]
    fn emulator(&self) -> &mut Apu {
        unsafe {
            &mut (*self.emulator)
        }
    }

    pub fn render_sample(&mut self, last_voice_out: i32, noise: i32, are_any_voices_solod: bool) -> VoiceOutput {
        let mut pitch = ((self.pitch_high as i32) << 8) | (self.pitch_low as i32);
        if self.pitch_mod {
            pitch += ((last_voice_out >> 5) * pitch) >> 10;
        }
        if pitch < 0 {
            pitch = 0;
        }
        if pitch > 0x3fff {
            pitch = 0x3fff;
        }

        let mut sample = if !self.noise_on {
            let s1 = self.resample_buffer[self.resample_buffer_pos];
            let s2 = self.resample_buffer[(self.resample_buffer_pos + 1) % RESAMPLE_BUFFER_LEN];
            let resampled = match self.resampling_mode {
                ResamplingMode::Linear => {
                    let p1 = self.sample_pos;
                    let p2 = 0x1000 - p1;
                    (s1 * p1 + s2 * p2) >> 12
                },
                ResamplingMode::Gaussian => {
                    let s3 = self.resample_buffer[(self.resample_buffer_pos + 2) % RESAMPLE_BUFFER_LEN];
                    let s4 = self.resample_buffer[(self.resample_buffer_pos + 3) % RESAMPLE_BUFFER_LEN];
                    let kernel_index = (self.sample_pos >> 2) as usize;
                    let p1 = HALF_KERNEL[kernel_index] as i32;
                    let p2 = HALF_KERNEL[kernel_index + HALF_KERNEL_SIZE / 2] as i32;
                    let p3 = HALF_KERNEL[HALF_KERNEL_SIZE - 1 - kernel_index] as i32;
                    let p4 = HALF_KERNEL[HALF_KERNEL_SIZE - 1 - (kernel_index + HALF_KERNEL_SIZE / 2)] as i32;
                    (s1 * p1 + s2 * p2 + s3 * p3 + s4 * p4) >> 11
                }
            };
            dsp_helpers::clamp(resampled) & !1
        } else {
            ((noise * 2) as i16) as i32
        };

        self.envelope.tick();
        let env_level = self.envelope.level;

        sample = ((sample * env_level) >> 11) & !1;

        if self.brr_block_decoder.is_end && !self.brr_block_decoder.is_looping {
            self.envelope.key_off();
            self.envelope.level = 0;
        }

        self.sample_pos += pitch;
        while self.sample_pos >= 0x1000 {
            self.sample_pos -= 0x1000;
            self.read_next_sample();

            if self.brr_block_decoder.is_finished() {
                if self.brr_block_decoder.is_end && self.brr_block_decoder.is_looping {
                    self.read_entry();
                    self.sample_address = self.loop_start_address;
                }
                self.read_next_block();
            }
        }

        let ret =
            if self.is_solod || (!self.is_muted && !are_any_voices_solod) {
                VoiceOutput {
                    left_out: dsp_helpers::multiply_volume(sample, self.vol_left),
                    right_out: dsp_helpers::multiply_volume(sample, self.vol_right),
                    last_voice_out: sample
                }
            } else {
                VoiceOutput {
                    left_out: 0,
                    right_out: 0,
                    last_voice_out: 0
                }
            };
        self.output_buffer.write(ret);
        ret
    }

    pub fn set_pitch_high(&mut self, value: u8) {
        self.pitch_high = value & 0x3f;
    }

    pub fn key_on(&mut self) {
        self.read_entry();
        self.sample_address = self.sample_start_address;
        self.brr_block_decoder.reset(0, 0);
        self.read_next_block();
        self.sample_pos = 0;
        for i in 0..RESAMPLE_BUFFER_LEN {
            self.resample_buffer[i] = 0;
        }
        self.read_next_sample();
        self.envelope.key_on();
    }

    pub fn key_off(&mut self) {
        self.envelope.key_off();
    }

    fn read_entry(&mut self) {
        self.sample_start_address = self.dsp().read_source_dir_start_address(self.source as i32);
        self.loop_start_address = self.dsp().read_source_dir_loop_address(self.source as i32);
    }

    fn read_next_block(&mut self) {
        let mut buf = [0; 9];
        for i in 0..9 {
            buf[i] = self.emulator().read_u8(self.sample_address + (i as u32));
        }
        self.brr_block_decoder.read(&buf);
        self.sample_address += 9;
    }

    fn read_next_sample(&mut self) {
        self.resample_buffer_pos = match self.resample_buffer_pos {
            0 => RESAMPLE_BUFFER_LEN - 1,
            x => x - 1
        };
        self.resample_buffer[self.resample_buffer_pos] = self.brr_block_decoder.read_next_sample() as i32;
    }
}
