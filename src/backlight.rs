//! Backlight policy for the 12 NeoPixels: inactivity timeout, host sleep, and
//! the manual off key.
//!
//! Deliberately free of HAL types - time arrives as a plain `u64` of
//! microseconds - so this is pure logic that can be reasoned about (and later
//! unit tested) without any hardware attached.

use smart_leds::RGB8;

/// No key pressed for this long and the LEDs fade out.
const IDLE_TIMEOUT_US: u64 = 5 * 60 * 1_000_000; // 5 minutes

/// How long the fade to black takes.
const FADE_US: u64 = 1_000_000; // 1 second

// Struct literals rather than RGB8::new, which is guaranteed usable in const position.
const WARM_WHITE: RGB8 = RGB8 { r: 255, g: 200, b: 120 };
const KEY_PRESS: RGB8 = RGB8 { r: 0, g: 255, b: 100 };

/// What the backlight is currently doing.
#[derive(Clone, Copy, PartialEq)]
enum Mode {
    /// Lit. The inactivity timer is running.
    On,
    /// Dark from inactivity or host sleep. Any key wakes it.
    Idle,
    /// Dark because the user pressed the off key. Only that key wakes it.
    Off,
}

pub struct Backlight {
    mode: Mode,
    last_activity_us: u64,
    dark_since_us: u64,
    suspended: bool,
}

impl Backlight {
    pub fn new(now_us: u64) -> Self {
        Self {
            mode: Mode::On,
            last_activity_us: now_us,
            dark_since_us: now_us,
            suspended: false,
        }
    }

    /// Any key is down.
    ///
    /// Deliberately does NOT rescue `Off`. That early return is what makes the
    /// manual off sticky, and what stops the activity check from undoing a
    /// key 12 toggle in the very same loop iteration.
    pub fn note_activity(&mut self, now_us: u64) {
        if self.mode == Mode::Off {
            return;
        }
        if self.mode == Mode::Idle {
            self.mode = Mode::On;
        }
        self.last_activity_us = now_us;
    }

    /// Key 12 rising edge. Any dark state wakes to `On` (the phone screen
    /// model, where the first press just wakes it); only `On` goes to `Off`.
    pub fn toggle(&mut self, now_us: u64) {
        match self.mode {
            Mode::On => {
                self.mode = Mode::Off;
                self.dark_since_us = now_us;
            }
            _ => {
                self.mode = Mode::On;
                self.last_activity_us = now_us;
            }
        }
    }

    /// Host suspend / resume.
    ///
    /// Acts on the EDGE only. `usb_dev.state()` reports `Suspend` on every loop
    /// while the host sleeps, so a level triggered version would reset
    /// `dark_since_us` hundreds of times a second and the fade would never
    /// progress past full brightness.
    pub fn set_suspended(&mut self, suspended: bool, now_us: u64) {
        if suspended == self.suspended {
            return;
        }
        self.suspended = suspended;

        if suspended {
            if self.mode == Mode::On {
                self.mode = Mode::Idle;
                self.dark_since_us = now_us;
            }
        } else {
            // Unconditional. The pad stays powered while the host sleeps, so
            // keys pressed at 3am can move `mode` out of `Idle` with no visible
            // effect (brightness is gated by `suspended`). That would leave a
            // stale `last_activity_us` behind, and the LEDs would snap on at
            // resume then fade straight back out. Waking the machine counts as
            // activity whatever state the pad drifted into.
            self.last_activity_us = now_us;
            if self.mode == Mode::Idle {
                self.mode = Mode::On;
            }
        }
    }

    /// Applies the inactivity timeout.
    pub fn tick(&mut self, now_us: u64) {
        if self.mode == Mode::On
            && !self.suspended
            && now_us.saturating_sub(self.last_activity_us) >= IDLE_TIMEOUT_US
        {
            self.mode = Mode::Idle;
            self.dark_since_us = now_us;
        }
    }

    /// 255 when lit, ramping to 0 over `FADE_US` once dark.
    ///
    /// Derived from elapsed time rather than accumulated per loop: the loop
    /// period is not constant (it shrinks once the strip stops being rewritten
    /// every iteration), so a per-iteration decrement would fade faster the
    /// darker it got.
    fn brightness(&self, now_us: u64) -> u8 {
        if self.mode == Mode::On && !self.suspended {
            return 255; // instant snap back on
        }
        let elapsed = now_us.saturating_sub(self.dark_since_us);
        if elapsed >= FADE_US {
            0
        } else {
            (255 - (elapsed * 255 / FADE_US)) as u8
        }
    }

    /// The buffer to hand to the strip.
    ///
    /// `pressed` carries all twelve keys including key 12, so LED 12 glows while
    /// the off key is held. That is press feedback on a key that sends no HID
    /// report and would otherwise feel dead.
    pub fn render(&self, now_us: u64, pressed: [bool; 12]) -> [RGB8; 12] {
        let level = self.brightness(now_us);
        let mut leds = [scale(WARM_WHITE, level); 12];

        if level > 0 {
            let lit = scale(KEY_PRESS, level);
            for (led, &down) in leds.iter_mut().zip(pressed.iter()) {
                if down {
                    *led = lit;
                }
            }
        }

        leds
    }
}

/// Scales a colour toward black. `level` 255 is untouched, 0 is off.
fn scale(colour: RGB8, level: u8) -> RGB8 {
    let s = |v: u8| ((v as u16 * level as u16) / 255) as u8;
    RGB8 {
        r: s(colour.r),
        g: s(colour.g),
        b: s(colour.b),
    }
}
