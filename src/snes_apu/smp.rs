pub struct Smp {
    // TODO: emulator
    reg_pc: u16,
    reg_a: u8,
    reg_x: u8,
    reg_y: u8,
    reg_sp: u8,

    psw_c: bool,
    psw_z: bool,
    psw_h: bool,
    psw_p: bool,
    psw_v: bool,
    psw_n: bool,

    // TODO: Look up some more behavior for I and B. Can't seem to find much
    // but some instructions DO set them.
    psw_i: bool,
    psw_b: bool,

    cycle_count: i32
}
