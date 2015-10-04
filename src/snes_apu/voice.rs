use super::brr_block_decoder::BrrBlockDecoder;

pub struct Voice {
    vol_left: u8,
    vol_right: u8,
    pitch_low: u8,
    pitch_high: u8,
    source: u8,
    pitch_mod: bool,
    noise_on: bool,
    echo_on: bool,

    sample_start_address: u32,
    loop_start_address: u32,
    brr_block_decoder: BrrBlockDecoder,
    sample_pos: i32,
    current_sample: i32,
    next_sample: i32
}
