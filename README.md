# RP2040 Macropad

my code (and future libraries) for custom macropad built with adafruit macropad rp2040 kit

## Config

Follow [adafruit tutorial](https://learn.adafruit.com/adafruit-macropad-rp2040) for adafruit macropad rp2040 kit with CircuitPython

Copy `code.py` and `/lib` to root of macropad, see tutorial for more info.

## Features

## Rotary Encoder

- Encoder: Volume Control
- Switch: Pause/Play

## Keys

0. Discord Mute (Custom keybind, bug with discord repeating keys)
1. None
2. Toggle between brightness modes (to expand later)

## TODO (no order)

- [ ] Build with rust
- [ ] Map out rest of keys
- [ ] Add more light modes
  - [ ] RGB
- [ ] Create profiles
- [ ] Use OLED more
- [ ] Check out [Macropad Hotkeys Project Guide](https://learn.adafruit.com/macropad-hotkeys) guide

## Bugs

- [x] Remove sleep causing unresponsiveness but maintain message duration
- [ ] Random reloads?

# Resources

- [Adafruit Macropad RP2040 Guide](https://learn.adafruit.com/adafruit-macropad-rp2040)
- [CircuitPython USB HID Driver Docs](https://docs.circuitpython.org/projects/hid/en/latest/index.html)
- [Adafruit MacroPad RP2040 Helper Library Docs](https://docs.circuitpython.org/projects/macropad/en/latest/index.html)
- [CircuitPython Docs](https://docs.circuitpython.org/en/latest/README.html)
- [Macropad Hotkeys Project Guide](https://learn.adafruit.com/macropad-hotkeys)
- [RP2040 MacOS Ventura Issue](https://www.raspberrypi.com/news/the-ventura-problem/)
