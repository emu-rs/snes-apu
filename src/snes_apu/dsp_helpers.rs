pub fn multiply_volume(value: i32, volume: u8) -> i32 {
    (value * (volume as i32)) >> 7
}

pub fn clamp(value: i32) -> i32 {
    if value < -32768 {
        return -32768;
    } else if value > 32767 {
        return 32767;
    }
    return value;
}
