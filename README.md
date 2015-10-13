# snes-apu [![Crates.io](https://img.shields.io/crates/v/snes-apu.svg)](https://crates.io/crates/snes-apu) [![Crates.io](https://img.shields.io/crates/l/emu.svg)](https://github.com/emu-rs/snes-apu/blob/master/LICENSE)
A Super Nintendo audio unit emulator.

## Description
This is a highly-accurate emulator for the audio portion of the Super Nintendo.

_Note: Due to current limitations in [the emu project](https://github.com/emu-rs/emu) on which this depends, this
will only build on osx. Other platforms will be supported promptly._

Included is an example that can play SPC files:

`cargo run --release --example spc_player -- test/ferris-nu.spc test/smashit.spc`

The audio unit is made up of a few major parts:
- A CPU (SPC700 core), which is 100% cycle-accurate and _shouldn't_ contain any bugs (unless some slipped in during the port)
- A DSP, which is accurate to the nearest audio sample
- 64kb RAM
- 3 timers, which are 100% accurate
- And some extra glue here and there :)

[Originally written in C++](https://github.com/yupferris/SamuraiPizzaCats), this emulator serves as a pilot for [porting all of my
emulator infrastructure to Rust](https://github.com/emu-rs/emu), and it's been a rather successful project thus far.

## Shortcomings
Currently, the code uses some unsafe code in a few places for internal mutability without runtime checks. Proper wrapping types
are also not currently used, so the emu can only run properly if built in the release config. Both of these issues will be addressed
at some point.

Additional issues can be found in the issue tracker on Github.

## Extras
Included in the `test` directory are a couple of test SPC files:
- `ferris-nu.spc` - soundtrack for ["nu" by elix](https://www.youtube.com/watch?v=wi-NxM1EaXM)
- `smashit.spc` - soundtrack for ["Smash It" by elix](https://www.youtube.com/watch?v=di_MnKNDfm0)

## Attribution
Much of the core SMP code was baked from byuu's higan source code: http://byuu.org/emulation/higan/

Likewise, some of the DSP code (envelopes in particular) was deep-fried from blargg's snes_spc code: http://blargg.8bitalley.com/libs/audio.html#snes_spc

Without their awesome work, this project wouldn't exist!

## License
This code is licensed under the BSD2 license (see LICENSE).
