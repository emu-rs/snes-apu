use super::super::apu::Apu;
use super::voice::{Voice, ResamplingMode};
use super::filter::Filter;
use super::ring_buffer::RingBuffer;
use super::super::spc::spc::{Spc, REG_LEN};
use super::dsp_helpers;

pub const SAMPLE_RATE: usize = 32000;
pub const BUFFER_LEN: usize = SAMPLE_RATE * 2;

const NUM_VOICES: usize = 8;

const COUNTER_RANGE: i32 = 30720;
static COUNTER_RATES: [i32; 32] = [
    COUNTER_RANGE + 1, // Never fires
    2048, 1536, 1280, 1024, 768, 640, 512, 384, 320, 256, 192, 160, 128, 96,
    80, 64, 48, 40, 32, 24, 20, 16, 12, 10, 8, 6, 5, 4, 3, 2, 1];

static COUNTER_OFFSETS: [i32; 32] = [
    1, 0, 1040, 536, 0, 1040, 536, 0, 1040, 536, 0, 1040, 536, 0, 1040,
    536, 0, 1040, 536, 0, 1040, 536, 0, 1040, 536, 0, 1040, 536, 0, 1040, 0, 0];

pub struct Dsp {
    emulator: *mut Apu,

    pub voices: Vec<Box<Voice>>,

    left_filter: Filter,
    right_filter: Filter,
    pub output_buffer: RingBuffer,

    vol_left: u8,
    vol_right: u8,
    echo_vol_left: u8,
    echo_vol_right: u8,
    noise_clock: u8,
    echo_write_enabled: bool,
    echo_feedback: u8,
    source_dir: u8,
    echo_start_address: u16,
    echo_delay: u8,

    counter: i32,

    cycles_since_last_flush: i32,
    is_flushing: bool,
    noise: i32,
    echo_pos: i32,
    echo_length: i32,

    resampling_mode: ResamplingMode
}

impl Dsp {
    pub fn new(emulator: *mut Apu) -> Box<Dsp> {
        let resampling_mode = ResamplingMode::Gaussian;
        let mut ret = Box::new(Dsp {
            emulator: emulator,

            voices: Vec::with_capacity(NUM_VOICES),

            left_filter: Filter::new(),
            right_filter: Filter::new(),
            output_buffer: RingBuffer::new(),

            vol_left: 0x89,
            vol_right: 0x9c,
            echo_vol_left: 0x9f,
            echo_vol_right: 0x9c,
            noise_clock: 0,
            echo_write_enabled: false,
            echo_feedback: 0,
            source_dir: 0,
            echo_start_address: Dsp::calculate_echo_start_address(0x60),
            echo_delay: 0x0e,

            counter: 0,

            cycles_since_last_flush: 0,
            is_flushing: false,
            noise: 0x4000,
            echo_pos: 0,
            echo_length: 0,

            resampling_mode: resampling_mode,
        });
        let ret_ptr = &mut *ret as *mut _;
        for _ in 0..NUM_VOICES {
            ret.voices.push(Box::new(Voice::new(ret_ptr, emulator, resampling_mode)));
        }
        ret.set_filter_coefficient(0x00, 0x80);
        ret.set_filter_coefficient(0x01, 0xff);
        ret.set_filter_coefficient(0x02, 0x9a);
        ret.set_filter_coefficient(0x03, 0xff);
        ret.set_filter_coefficient(0x04, 0x67);
        ret.set_filter_coefficient(0x05, 0xff);
        ret.set_filter_coefficient(0x06, 0x0f);
        ret.set_filter_coefficient(0x07, 0xff);
        ret.set_resampling_mode(ResamplingMode::Gaussian);
        ret
    }

    #[inline]
    fn emulator(&self) -> &mut Apu {
        unsafe {
            &mut (*self.emulator)
        }
    }

    fn set_filter_coefficient(&mut self, index: i32, value: u8) {
        self.left_filter.coefficients[index as usize] = value;
        self.right_filter.coefficients[index as usize] = value;
    }

    pub fn resampling_mode(&self) -> ResamplingMode {
        self.resampling_mode
    }

    pub fn set_resampling_mode(&mut self, resampling_mode: ResamplingMode) {
        self.resampling_mode = resampling_mode;
        for voice in self.voices.iter_mut() {
            voice.resampling_mode = resampling_mode;
        }
    }

    fn calculate_echo_start_address(value: u8) -> u16 {
        (value as u16) << 8
    }

    pub fn set_state(&mut self, spc: &Spc) {
        for i in 0..REG_LEN {
            match i {
                0x4c | 0x5c => (), // Do nothing
                _ => { self.set_register(i as u8, spc.regs[i as usize]); }
            }
        }

        self.set_kon(spc.regs[0x4c]);
    }

    pub fn cycles_callback(&mut self, num_cycles: i32) {
        self.cycles_since_last_flush += num_cycles;
    }

    pub fn get_echo_start_address(&self) -> u16 {
        self.echo_start_address
    }

    pub fn calculate_echo_length(&self) -> i32 {
        (self.echo_delay as i32) * 0x800
    }

    pub fn flush(&mut self) {
        self.is_flushing = true;

        while self.cycles_since_last_flush > 64 {
            if !self.read_counter(self.noise_clock as i32) {
                let feedback = (self.noise << 13) ^ (self.noise << 14);
                self.noise = (feedback & 0x4000) ^ (self.noise >> 1);
            }

            let mut are_any_voices_solod = false;
            for voice in self.voices.iter() {
                if voice.is_solod {
                    are_any_voices_solod = true;
                    break;
                }
            }

            let mut left_out = 0;
            let mut right_out = 0;
            let mut left_echo_out = 0;
            let mut right_echo_out = 0;
            let mut last_voice_out = 0;
            for voice in self.voices.iter_mut() {
                let output = voice.render_sample(last_voice_out, self.noise, are_any_voices_solod);

                left_out = dsp_helpers::clamp(left_out + output.left_out);
                right_out = dsp_helpers::clamp(right_out + output.right_out);

                if voice.echo_on {
                    left_echo_out = dsp_helpers::clamp(left_echo_out + output.left_out);
                    right_echo_out = dsp_helpers::clamp(right_echo_out + output.right_out);
                }

                last_voice_out = output.last_voice_out;
            }

            left_out = dsp_helpers::multiply_volume(left_out, self.vol_left);
            right_out = dsp_helpers::multiply_volume(right_out, self.vol_right);

            let echo_address = (self.echo_start_address + (self.echo_pos as u16)) as u32;
            let mut left_echo_in = (((((self.emulator().read_u8(echo_address + 1) as i32) << 8) | (self.emulator().read_u8(echo_address) as i32)) as i16) & !1) as i32;
            let mut right_echo_in = (((((self.emulator().read_u8(echo_address + 3) as i32) << 8) | (self.emulator().read_u8(echo_address + 2) as i32)) as i16) & !1) as i32;

            left_echo_in = dsp_helpers::clamp(self.left_filter.next(left_echo_in));
            right_echo_in = dsp_helpers::clamp(self.right_filter.next(right_echo_in));

            let left_out = dsp_helpers::clamp(left_out + dsp_helpers::multiply_volume(left_echo_in, self.echo_vol_left)) as i16;
            let right_out = dsp_helpers::clamp(right_out + dsp_helpers::multiply_volume(right_echo_in, self.echo_vol_right)) as i16;
            self.output_buffer.write_sample(left_out, right_out);

            if self.echo_write_enabled {
                left_echo_out = dsp_helpers::clamp(left_echo_out + ((((left_echo_in * ((self.echo_feedback as i8) as i32)) >> 7) as i16) as i32)) & !1;
                right_echo_out = dsp_helpers::clamp(right_echo_out + ((((right_echo_in * ((self.echo_feedback as i8) as i32)) >> 7) as i16) as i32)) & !1;

                self.emulator().write_u8(echo_address + 0, left_echo_out as u8);
                self.emulator().write_u8(echo_address + 1, (left_echo_out >> 8) as u8);
                self.emulator().write_u8(echo_address + 2, right_echo_out as u8);
                self.emulator().write_u8(echo_address + 3, (right_echo_out >> 8) as u8);
            }
            if self.echo_pos == 0 {
                self.echo_length = self.calculate_echo_length();
            }
            self.echo_pos += 4;
            if self.echo_pos >= self.echo_length {
                self.echo_pos = 0;
            }

            self.counter = (self.counter + 1) % COUNTER_RANGE;
            self.cycles_since_last_flush -= 64;
        }

        self.is_flushing = false;
    }

    pub fn set_register(&mut self, address: u8, value: u8) {
        if (address & 0x80) != 0 {
            return;
        }

        if !self.is_flushing {
            self.flush();
        }

        let voice_index = address >> 4;
        let voice_address = address & 0x0f;
        if voice_address < 0x0a {
            if voice_address < 8 {
                let voice = &mut self.voices[voice_index as usize];
                match voice_address {
                    0x00 => { voice.vol_left = value; },
                    0x01 => { voice.vol_right = value; },
                    0x02 => { voice.pitch_low = value; },
                    0x03 => { voice.set_pitch_high(value); },
                    0x04 => { voice.source = value; },
                    0x05 => { voice.envelope.adsr0 = value; },
                    0x06 => { voice.envelope.adsr1 = value; },
                    0x07 => { voice.envelope.gain = value; },
                    _ => () // Do nothing
                }
            }
        } else if voice_address == 0x0f {
            self.set_filter_coefficient(voice_index as i32, value);
        } else {
            match address {
                0x0c => { self.vol_left = value; },
                0x1c => { self.vol_right = value; },
                0x2c => { self.echo_vol_left = value; },
                0x3c => { self.echo_vol_right = value; },
                0x4c => { self.set_kon(value); },
                0x5c => { self.set_kof(value); },
                0x6c => { self.set_flg(value); },

                0x0d => { self.echo_feedback = value; },

                0x2d => { self.set_pmon(value); },
                0x3d => { self.set_nov(value); },
                0x4d => { self.set_eon(value); },
                0x5d => { self.source_dir = value; },
                0x6d => { self.echo_start_address = (value as u16) << 8; },
                0x7d => { self.echo_delay = value & 0x0f; },

                _ => () // Do nothing
            }
        }
    }

    pub fn get_register(&mut self, address: u8) -> u8 {
        if !self.is_flushing {
            self.flush();
        }

        let _ = address;
        0
    }

    pub fn read_counter(&self, rate: i32) -> bool {
        ((self.counter + COUNTER_OFFSETS[rate as usize]) % COUNTER_RATES[rate as usize]) != 0
    }

    pub fn read_source_dir_start_address(&self, index: i32) -> u32 {
        self.read_source_dir_address(index, 0)
    }

    pub fn read_source_dir_loop_address(&self, index: i32) -> u32 {
        self.read_source_dir_address(index, 2)
    }

    fn read_source_dir_address(&self, index: i32, offset: i32) -> u32 {
        let dir_address = (self.source_dir as i32) * 0x100;
        let entry_address = dir_address + index * 4;
        let mut ret = self.emulator().read_u8((entry_address as u32) + (offset as u32)) as u32;
        ret |= (self.emulator().read_u8((entry_address as u32) + (offset as u32) + 1) as u32) << 8;
        ret
    }

    fn set_kon(&mut self, voice_mask: u8) {
        for i in 0..NUM_VOICES {
            if ((voice_mask as usize) & (1 << i)) != 0 {
                self.voices[i].key_on();
            }
        }
    }

    fn set_kof(&mut self, voice_mask: u8) {
        for i in 0..NUM_VOICES {
            if ((voice_mask as usize) & (1 << i)) != 0 {
                self.voices[i].key_off();
            }
        }
    }

    fn set_flg(&mut self, value: u8) {
        self.noise_clock = value & 0x1f;
        self.echo_write_enabled = (value & 0x20) == 0;
    }

    fn set_pmon(&mut self, voice_mask: u8) {
        for i in 1..NUM_VOICES {
            self.voices[i].pitch_mod = ((voice_mask as usize) & (1 << i)) != 0;
        }
    }

    fn set_nov(&mut self, voice_mask: u8) {
        for i in 0..NUM_VOICES {
            self.voices[i].noise_on = ((voice_mask as usize) & (1 << i)) != 0;
        }
    }

    fn set_eon(&mut self, voice_mask: u8) {
        for i in 0..NUM_VOICES {
            self.voices[i].echo_on = ((voice_mask as usize) & (1 << i)) != 0;
        }
    }
}
