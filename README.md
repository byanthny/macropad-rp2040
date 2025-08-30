# RP2040 Macropad w/ Rust (Adafruit Kit) [use at your own risk]

adafruit macropad rp2040 kit programmed with rust

## Setup

...rust setup

1. Build project with:

```bash
cargo build
```

2. Put the RP2040 into bootloader mode.
3. Flash macropad with:

```bash
cargo run --release
```

## Features

### Rotary Encoder

- Encoder: Volume Control [not implemented]
- Switch: Toggle between brightness modes [not implemented]

### Keys

1. A
2. Pause/Play
3. Skip
4. Reject (lightroom)
5. Unflagged (lightroom)
6. Pick (lightroom)
7.
8.
9.
10.
11.
12.

## TODO (no order)

- [ ] ~~Reintroduce previous features~~ Figure out useful keys
- [ ] Add rotary encoder functionality
- [ ] Add LED support
- [ ] Add OLED display support
- [ ] Explore profiles and switching key mappings
- [ ] LED effects
- [ ] Clean up project
- [ ] Use Enums or Trait Objects

## Notes & Bugs

- usbd_hid crate was giving me issues with keyboard input not being recognized, used usbd-human-interface-device instead

# Resources

- [Docs.rs](https://docs.rs/)
- [Adafruit Macropad RP2040 Guide](https://learn.adafruit.com/adafruit-macropad-rp2040)
- [Adafruit MacroPad Datasheet](https://github.com/adafruit/Adafruit-MacroPad-RP2040-PCB/blob/fdd7f2cb3bc2b3c7a9c0765780387647ea872141/Adafruit%20MacroPad%20RP2040%20Pinout.pdf)
- [RP2040 HAL Template (pins are different used for setup)](https://github.com/rp-rs/rp2040-project-template)
- [Adafruit Macropad BSP](https://lib.rs/crates/adafruit-macropad)
- [BSP LED Blink Example](https://github.com/rp-rs/rp-hal-boards/blob/56e044061073fb49aef93984b629af5c5bc1a11c/boards/adafruit-macropad/examples/adafruit-macropad_blinky.rs)
- [usbd-human-interface-device](https://docs.rs/usbd-human-interface-device/)
