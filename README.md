# snes-apu [![Build Status](https://travis-ci.org/emu-rs/snes-apu.svg)](https://travis-ci.org/emu-rs/snes-apu) [![Crates.io](https://img.shields.io/crates/v/snes-apu.svg)](https://crates.io/crates/snes-apu) [![Crates.io](https://img.shields.io/crates/l/emu.svg)](https://github.com/emu-rs/snes-apu/blob/master/LICENSE)
A Super Nintendo audio unit emulator.

## Description
This is a highly-accurate emulator for the audio portion of the Super Nintendo.

Included is an example that can play SPC files:

`cargo run --release --example spc_player -- test/ferris-nu.spc test/smashit.spc`

_Note: Due to current limitations in [the emu project](https://github.com/emu-rs/emu) on which this depends, the included spc_player example will only build on OS X. Other platforms will be supported promptly._

The audio unit is made up of a few major parts:
- A CPU (SPC700 core), which is 100% cycle-accurate
- A DSP, which is accurate to the nearest audio sample
- 64kb RAM
- 3 timers
- And some extra glue here and there to tie it all together :)

[Originally written in C++](https://github.com/yupferris/SamuraiPizzaCats), this emulator serves as a pilot for [porting all of my
emulator infrastructure to Rust](https://github.com/emu-rs/emu), and it's been a rather successful project thus far.

## Shortcomings
While the emulation itself is highly-accurate (no known bugs minus some things that aren't implemented fully, such as DSP register reads
and startup state), and the vast majority of the code can be considered idiomatic Rust code (to the best of my knowledge), there are some
small remaining "cosmetic issues" that need to be addressed.

Namely, some unsafe code is used in a few places for internal mutability without runtime checks. Proper wrapping number types are also not
currently used, so the emu can only run properly if built in the release config. Both of these issues will be addressed soon.

Additional issues can be found in the issue tracker on Github.

## Extras
Included in the `test` directory are a couple of test SPC files:
- `ferris-nu.spc` - soundtrack for ["nu" by elix](https://www.youtube.com/watch?v=wi-NxM1EaXM)
- `smashit.spc` - soundtrack for ["Smash It" by elix](https://www.youtube.com/watch?v=di_MnKNDfm0)

Other projects consuming this library:
- [snes-apu-dbg](https://github.com/yupferris/snes-apu-dbg) - a Qt-based graphical debugger used in development of this library

## Attribution
Much of the core SMP code was baked from byuu's higan source code: http://byuu.org/emulation/higan/

Likewise, some of the DSP code (envelopes in particular) was deep-fried from blargg's snes_spc code: http://blargg.8bitalley.com/libs/audio.html#snes_spc

Without their awesome work, this project wouldn't exist!

## License
This code is licensed under the BSD2 license (see LICENSE).
