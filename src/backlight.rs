//! Backlight policy for the 12 NeoPixels: inactivity timeout, host sleep, and
//! the manual off key.
//!
//! Deliberately free of HAL types - time arrives as a plain `u64` of
//! microseconds - so this is pure logic that can be reasoned about (and later
//! unit tested) without any hardware attached.

use smart_leds::RGB8;

/// No key pressed for this long and the LEDs fade out.
const IDLE_TIMEOUT_US: u64 = 5 * 60 * 1_000_000; // 5 minutes

/// Going dark is unhurried, so it reads as the pad settling rather than a fault.
const FADE_OUT_US: u64 = 1_000_000; // 1 second

/// Coming back is quicker, so the pad feels responsive under the hand.
const FADE_IN_US: u64 = 250_000; // 250ms

/// Brightness the pad sits at when lit, out of 255. Half scale: the full range
/// is glaring on a desk at night.
const MAX_LEVEL: u8 = 128;

// Struct literals rather than RGB8::new, which is guaranteed usable in const position.
// These are the hues at full scale; MAX_LEVEL sets how bright they actually burn.
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
    suspended: bool,
    /// Level the current ramp started from, and when it started. Together these
    /// let `brightness()` be a pure function of the clock.
    fade_from: u8,
    fade_start_us: u64,
}

impl Backlight {
    /// Starts dark and ramps up, so the pad visibly comes to life on plug-in.
    pub fn new(now_us: u64) -> Self {
        Self {
            mode: Mode::On,
            last_activity_us: now_us,
            suspended: false,
            fade_from: 0,
            fade_start_us: now_us,
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
            self.begin_fade(now_us);
            self.mode = Mode::On;
        }
        self.last_activity_us = now_us;
    }

    /// Key 12 rising edge. Any dark state wakes to `On` (the phone screen
    /// model, where the first press just wakes it); only `On` goes to `Off`.
    pub fn toggle(&mut self, now_us: u64) {
        self.begin_fade(now_us);
        match self.mode {
            Mode::On => self.mode = Mode::Off,
            _ => {
                self.mode = Mode::On;
                self.last_activity_us = now_us;
            }
        }
    }

    /// Host suspend / resume.
    ///
    /// Acts on the EDGE only. `usb_dev.state()` reports `Suspend` on every loop
    /// while the host sleeps, so a level triggered version would restart the
    /// ramp hundreds of times a second and the fade would never progress.
    pub fn set_suspended(&mut self, suspended: bool, now_us: u64) {
        if suspended == self.suspended {
            return;
        }
        self.begin_fade(now_us);
        self.suspended = suspended;

        if suspended {
            if self.mode == Mode::On {
                self.mode = Mode::Idle;
            }
        } else {
            // Unconditional. The pad stays powered while the host sleeps, so
            // keys pressed at 3am can move `mode` out of `Idle` with no visible
            // effect (the target level is gated by `suspended`). That would
            // leave a stale `last_activity_us` behind, and the LEDs would come
            // up at resume then fade straight back out. Waking the machine
            // counts as activity whatever state the pad drifted into.
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
            self.begin_fade(now_us);
            self.mode = Mode::Idle;
        }
    }

    /// Where the backlight is heading right now.
    fn target(&self) -> u8 {
        if self.mode == Mode::On && !self.suspended {
            MAX_LEVEL
        } else {
            0
        }
    }

    /// Freezes the currently visible level as the start of a new ramp.
    ///
    /// MUST be called before changing `mode` or `suspended`, since it reads the
    /// brightness the old state was showing. Starting from the visible level
    /// rather than from the extreme is what lets an interrupted fade reverse
    /// smoothly instead of jumping.
    fn begin_fade(&mut self, now_us: u64) {
        self.fade_from = self.brightness(now_us);
        self.fade_start_us = now_us;
    }

    /// Linear ramp from `fade_from` toward the target.
    ///
    /// Derived from elapsed time rather than accumulated per loop: the loop
    /// period is not constant (it shrinks once the strip stops being rewritten
    /// every iteration), so a per-iteration step would fade faster the darker
    /// it got.
    fn brightness(&self, now_us: u64) -> u8 {
        let target = self.target();
        if target == self.fade_from {
            return target;
        }

        let rising = target > self.fade_from;
        let duration = if rising { FADE_IN_US } else { FADE_OUT_US };
        let elapsed = now_us.saturating_sub(self.fade_start_us);
        if elapsed >= duration {
            return target;
        }

        let span = if rising {
            target - self.fade_from
        } else {
            self.fade_from - target
        };
        let travelled = (span as u64 * elapsed / duration) as u8;

        if rising {
            self.fade_from + travelled
        } else {
            self.fade_from - travelled
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
