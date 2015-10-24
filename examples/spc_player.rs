extern crate emu;
extern crate spc;
extern crate snes_apu;

use std::iter;
use std::env;
use std::io::{Result, Error, ErrorKind, Write, stdout, stdin};
use std::thread;
use std::sync::mpsc::{channel, Sender, Receiver};

use emu::audio_driver::AudioDriver;
use emu::audio_driver_factory;

use spc::spc::{Spc, Emulator};
use snes_apu::apu::Apu;
use snes_apu::dsp::dsp::{SAMPLE_RATE, BUFFER_LEN};

fn main() {
    if let Err(e) = play_spc_files() {
        println!("ERROR: {}", e);
        std::process::exit(1);
    }
}

fn play_spc_files() -> Result<()> {
    let mut driver = audio_driver_factory::create_default();
    driver.set_sample_rate(SAMPLE_RATE as i32);

    let (is_done_send, is_done_recv) = channel();

    spawn_keypress_thread(is_done_send.clone());

    for file_name in try!(get_file_names()) {
        try!(play_spc_file(&mut driver, &is_done_send, &is_done_recv, &file_name));
    }

    Ok(())
}

fn get_file_names() -> Result<iter::Skip<env::Args>> {
    let args = env::args();
    match args.len() {
        1 => Err(Error::new(ErrorKind::Other, "No file(s) specified")),
        _ => Ok(args.skip(1))
    }
}

fn spawn_keypress_thread(is_done_send: Sender<()>) {
    thread::spawn(move || {
        loop {
            let mut s = String::new();
            stdin().read_line(&mut s).ok();
            is_done_send.send(()).ok();
        }
    });
}

struct SpcEndState {
    sample_pos: i32,
    fade_out_sample: i32,
    end_sample: i32,
    is_done_send: Sender<()>
}

fn play_spc_file(driver: &mut Box<AudioDriver>, is_done_send: &Sender<()>, is_done_recv: &Receiver<()>, file_name: &String) -> Result<()> {
    let spc = try!(Spc::load(file_name));

    print_spc_file_info(file_name, &spc);

    let mut apu = Apu::new();
    apu.set_state(&spc);
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
            is_done_send: is_done_send.clone()
        })
    } else {
        None
    };

    driver.set_render_callback(Some(Box::new(move |buffer, num_frames| {
        apu.render(&mut *left, &mut *right, num_frames as i32);
        match end_state {
            Some(ref mut state) => {
                for i in 0..num_frames {
                    let j = i * 2;
                    let sample_index = state.sample_pos + (i as i32);
                    let f = if sample_index >= state.end_sample {
                        0.0
                    } else if sample_index >= state.fade_out_sample {
                        1.0 - ((sample_index - state.fade_out_sample) as f32) / ((state.end_sample - state.fade_out_sample) as f32)
                    } else {
                        1.0
                    };
                    buffer[j + 0] = left[i] as f32 * f / 32768.0;
                    buffer[j + 1] = right[i] as f32 * f / 32768.0;
                }
                state.sample_pos += num_frames as i32;
                if state.sample_pos >= state.end_sample {
                    state.is_done_send.send(()).ok();
                }
            },
            _ => {
                for i in 0..num_frames {
                    let j = i * 2;
                    buffer[j + 0] = left[i] as f32 / 32768.0;
                    buffer[j + 1] = right[i] as f32 / 32768.0;
                }
            }
        }
    })));

    wait_for_key_press_with_busy_icon(&is_done_recv)
}

fn print_spc_file_info(file_name: &String, spc: &Spc) {
    println!("SPC: {}", file_name);
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
            Emulator::Snes9x => "Snes9x"
        });
    } else {
        println!(" No ID666 tag present.");
    };
}

fn wait_for_key_press_with_busy_icon(is_done_recv: &Receiver<()>) -> Result<()> {
    println!("Return stops song.");
    let chars = ['-', '/', '|', '\\'];
    let mut char_index = 0;
    loop {
        if let Ok(()) = is_done_recv.try_recv() {
            break;
        }

        print!("\r[{}]", chars[char_index]);
        stdout().flush().ok();
        char_index = (char_index + 1) % chars.len();

        thread::sleep_ms(5);
    }
    print!("\r   \r");

    Ok(())
}
