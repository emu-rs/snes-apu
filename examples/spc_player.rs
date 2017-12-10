extern crate cpal;
extern crate futures;
extern crate snes_apu;
extern crate spc;

use cpal::{EventLoop, Voice, UnknownTypeBuffer, default_endpoint};

use futures::stream::Stream;
use futures::task::{self, Executor, Run};

use snes_apu::apu::Apu;
use snes_apu::dsp::dsp::{BUFFER_LEN, SAMPLE_RATE};

use spc::spc::{Emulator, Spc};

use std::borrow::Cow;
use std::env;
use std::fmt::Display;
use std::io::{stdout, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

struct SpcEndState {
    sample_pos: i32,
    fade_out_sample: i32,
    end_sample: i32,
}

fn main() {
    if let Err(e) = do_it() {
        println!("ERROR: {}", e);
        std::process::exit(1);
    }
}

fn do_it() -> Result<(), Cow<'static, str>> {
    let mut args = env::args();
    let path = match args.len() {
        0 | 1 => Err("No file specified"),
        2 => Ok(args.nth(1).unwrap()),
        _ => Err("Only one file argument can be specified"),
    }?;
    play_spc_file(path)
}

fn play_spc_file<P: AsRef<Path> + Display>(path: P) -> Result<(), Cow<'static, str>> {
    let spc = Spc::load(&path).map_err(|e| format!("Could not load spc file: {}", e))?;

    print_spc_info(path, &spc);

    let mut apu = Apu::from_spc(&spc);
    // Most SPC's have crap in the echo buffer on startup, so while it's not technically correct, we'll clear that.
    // The example for blargg's APU emulator (which is known to be the most accurate there is) also does this, so I
    //  think we're OK to do it too :)
    apu.clear_echo_buffer();

    let mut left = Box::new([0; BUFFER_LEN]);
    let mut right = Box::new([0; BUFFER_LEN]);

    let mut end_state = if let Some(ref id666_tag) = spc.id666_tag {
        let fade_out_sample = id666_tag.seconds_to_play_before_fading_out * (SAMPLE_RATE as i32);
        let end_sample = fade_out_sample + id666_tag.fade_out_length * (SAMPLE_RATE as i32) / 1000;
        Some(SpcEndState {
            sample_pos: 0,
            fade_out_sample: fade_out_sample,
            end_sample: end_sample,
        })
    } else {
        None
    };

    let driver = CpalDriver::new(SAMPLE_RATE as _, 100)?;

    let chars = ['-', '/', '|', '\\'];
    let mut char_index = 0;
    loop {
        {
            let mut ring_buffer = driver.ring_buffer.lock().unwrap();

            let num_frames = ((ring_buffer.samples_read - ring_buffer.samples_written) as u32) / 2;
            apu.render(&mut *left, &mut *right, num_frames as i32);

            match end_state {
                Some(ref mut state) => {
                    for i in 0..num_frames {
                        let sample_index = state.sample_pos + (i as i32);
                        let f = if sample_index >= state.end_sample {
                            0.0
                        } else if sample_index >= state.fade_out_sample {
                            1.0 - ((sample_index - state.fade_out_sample) as f32) / ((state.end_sample - state.fade_out_sample) as f32)
                        } else {
                            1.0
                        };
                        ring_buffer.push(((left[i as usize] as f32) * f) as _);
                        ring_buffer.push(((right[i as usize] as f32) * f) as _);
                    }
                    state.sample_pos += num_frames as i32;
                    if state.sample_pos >= state.end_sample {
                        break;
                    }
                },
                _ => {
                    for i in 0..num_frames {
                        ring_buffer.push(left[i as usize]);
                        ring_buffer.push(right[i as usize]);
                    }
                }
            }
        }

        print!("\r[{}]", chars[char_index]);
        stdout().flush().ok();
        char_index = (char_index + 1) % chars.len();

        thread::sleep(Duration::from_millis(5));
    }

    Ok(())
}

fn print_spc_info<P: AsRef<Path> + Display>(path: P, spc: &Spc) {
    println!("SPC: {}", path);
    println!(" Version Minor: {}", spc.version_minor);
    println!(" PC: {}", spc.pc);
    println!(" A: {}", spc.a);
    println!(" X: {}", spc.x);
    println!(" Y: {}", spc.y);
    println!(" PSW: {}", spc.psw);
    println!(" SP: {}", spc.sp);

    if let Some(ref id666_tag) = spc.id666_tag {
        println!(" ID666 tag present:");
        println!("  Song title: {}", id666_tag.song_title);
        println!("  Game title: {}", id666_tag.game_title);
        println!("  Dumper name: {}", id666_tag.dumper_name);
        println!("  Comments: {}", id666_tag.comments);
        println!("  Date dumped (MM/DD/YYYY): {}", id666_tag.date_dumped);
        println!("  Seconds to play before fading out: {}", id666_tag.seconds_to_play_before_fading_out);
        println!("  Fade out length: {}ms", id666_tag.fade_out_length);
        println!("  Artist name: {}", id666_tag.artist_name);
        println!("  Default channel disables: {}", id666_tag.default_channel_disables);
        println!("  Dumping emulator: {}", match id666_tag.dumping_emulator {
            Emulator::Unknown => "Unknown",
            Emulator::ZSnes => "ZSnes",
            Emulator::Snes9x => "Snes9x",
        });
    } else {
        println!(" No ID666 tag present.");
    };
}

pub struct RingBuffer {
    inner: Box<[i16]>,

    write_pos: usize,
    read_pos: usize,

    samples_written: u64,
    samples_read: u64,
}

impl RingBuffer {
    fn push(&mut self, value: i16) {
        self.inner[self.write_pos] = value;

        self.write_pos += 1;
        if self.write_pos >= self.inner.len() {
            self.write_pos = 0;
        }

        self.samples_written += 1;
    }
}

impl Iterator for RingBuffer {
    type Item = i16;

    fn next(&mut self) -> Option<i16> {
        let ret = self.inner[self.read_pos];

        self.read_pos += 1;
        if self.read_pos >= self.inner.len() {
            self.read_pos = 0;
        }

        self.samples_read += 1;

        Some(ret)
    }
}

struct CpalDriverExecutor;

impl Executor for CpalDriverExecutor {
    fn execute(&self, r: Run) {
        r.run();
    }
}

pub struct CpalDriver {
    ring_buffer: Arc<Mutex<RingBuffer>>,

    _voice: Voice,
    _render_thread_join_handle: JoinHandle<()>,
}

impl CpalDriver {
    pub fn new(sample_rate: u32, desired_latency_ms: u32) -> Result<CpalDriver, Cow<'static, str>> {
        if desired_latency_ms == 0 {
            return Err(format!("desired_latency_ms must be greater than 0").into());
        }

        let endpoint = default_endpoint().ok_or("Failed to get audio endpoint")?;

        let format = endpoint.supported_formats()
            .map_err(|e| format!("Failed to get supported format list for endpoint: {}", e))?
            .find(|format| format.channels.len() == 2)
            .ok_or("Failed to find format with 2 channels")?;

        let buffer_frames = sample_rate * desired_latency_ms / 1000 * 2;
        let ring_buffer = Arc::new(Mutex::new(RingBuffer {
            inner: vec![0; buffer_frames as usize].into_boxed_slice(),

            write_pos: 0,
            read_pos: 0,

            samples_written: 0,
            samples_read: 0,
        }));

        let event_loop = EventLoop::new();

        let (mut voice, stream) = Voice::new(&endpoint, &format, &event_loop).map_err(|e| format!("Failed to create voice: {}", e))?;
        voice.play();

        let mut resampler = LinearResampler::new(sample_rate as _, format.samples_rate.0 as _);

        let read_ring_buffer = ring_buffer.clone();
        task::spawn(stream.for_each(move |output_buffer| {
            let mut read_ring_buffer = read_ring_buffer.lock().unwrap();

            match output_buffer {
                UnknownTypeBuffer::I16(mut buffer) => {
                    for sample in buffer.chunks_mut(format.channels.len()) {
                        for out in sample.iter_mut() {
                            *out = resampler.next(&mut *read_ring_buffer);
                        }
                    }
                },
                UnknownTypeBuffer::U16(mut buffer) => {
                    for sample in buffer.chunks_mut(format.channels.len()) {
                        for out in sample.iter_mut() {
                            *out = ((resampler.next(&mut *read_ring_buffer) as isize) + 32768) as u16;
                        }
                    }
                },
                UnknownTypeBuffer::F32(mut buffer) => {
                    for sample in buffer.chunks_mut(format.channels.len()) {
                        for out in sample.iter_mut() {
                            *out = (resampler.next(&mut *read_ring_buffer) as f32) / 32768.0;
                        }
                    }
                },
            }

            Ok(())
        })).execute(Arc::new(CpalDriverExecutor));

        let render_thread_join_handle = thread::spawn(move || {
            event_loop.run();
        });

        Ok(CpalDriver {
            ring_buffer: ring_buffer,

            _voice: voice,
            _render_thread_join_handle: render_thread_join_handle,
        })
    }
}

struct LinearResampler {
    from_sample_rate: u32,
    to_sample_rate: u32,

    current_from_frame: [i16; 2],
    next_from_frame: [i16; 2],
    from_fract_pos: u32,

    current_frame_channel_offset: u32,
}

impl LinearResampler {
    fn new(from_sample_rate: u32, to_sample_rate: u32) -> LinearResampler {
        let sample_rate_gcd = {
            fn gcd(a: u32, b: u32) -> u32 {
                if b == 0 {
                    a
                } else {
                    gcd(b, a % b)
                }
            }

            gcd(from_sample_rate, to_sample_rate)
        };

        LinearResampler {
            from_sample_rate: from_sample_rate / sample_rate_gcd,
            to_sample_rate: to_sample_rate / sample_rate_gcd,

            current_from_frame: [0, 0],
            next_from_frame: [0, 0],
            from_fract_pos: 0,

            current_frame_channel_offset: 0,
        }
    }

    fn next(&mut self, input: &mut Iterator<Item = i16>) -> i16 {
        fn interpolate(a: i16, b: i16, num: u32, denom: u32) -> i16 {
            (((a as i32) * ((denom - num) as i32) + (b as i32) * (num as i32)) / (denom as i32)) as _
        }

        let ret = match self.current_frame_channel_offset {
            0 => interpolate(self.current_from_frame[0], self.next_from_frame[0], self.from_fract_pos, self.to_sample_rate),
            _ => interpolate(self.current_from_frame[1], self.next_from_frame[1], self.from_fract_pos, self.to_sample_rate)
        };

        self.current_frame_channel_offset += 1;
        if self.current_frame_channel_offset >= 2 {
            self.current_frame_channel_offset = 0;

            self.from_fract_pos += self.from_sample_rate;
            while self.from_fract_pos > self.to_sample_rate {
                self.from_fract_pos -= self.to_sample_rate;

                self.current_from_frame = self.next_from_frame;

                let left = input.next().unwrap_or(0);
                let right = input.next().unwrap_or(0);
                self.next_from_frame = [left, right];
            }
        }

        ret
    }
}
