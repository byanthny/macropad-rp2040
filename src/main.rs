// #[] gives information to the compiler
#![no_std]
#![no_main]

//import panic handler (other options are panic_abort, panic_reset, ...)
//"as _" imported/linked w/o name
use panic_halt as _; 

// Import the entry macro from cortex-m runtime
//This macro knows how to properly set up an ARM Cortex-M program.
use cortex_m_rt::entry;

// hardware access
use embedded_hal::digital::{InputPin, OutputPin};
use adafruit_macropad::{
    hal::{
        clocks::{init_clocks_and_plls, Clock},
        pac,
        watchdog::Watchdog,
        Sio,
        Timer,
        usb::UsbBus,
        pio::PIOExt,
    },
    Pins, XOSC_CRYSTAL_FREQ,
};

// USB HID imports
use usb_device::{prelude::*, class_prelude::*, device::UsbDeviceState};
use usbd_human_interface_device::{prelude::*, page::{Keyboard, Consumer}, device::{keyboard::{NKROBootKeyboardConfig, NKROBootKeyboard}, consumer::{ConsumerControlConfig, ConsumerControl, MultipleConsumerReport}}}; //MultipleConsumerReport has 4 consumer control codes

// NeoPixel LED imports
use smart_leds::RGB8;
use adafruit_macropad::hal::pio::{PIOBuilder, Tx, ValidStateMachine};
use adafruit_macropad::hal::gpio::FunctionPio0;

// Backlight policy: inactivity timeout, host sleep, manual off key
mod backlight;
use backlight::Backlight;

/// Ignore further key 12 edges for this long after a toggle.
/// Models switch contact bounce, so it lives here next to the pin read rather
/// than inside the backlight state machine.
const TOGGLE_LOCKOUT_US: u64 = 150_000; // 150ms

/// Rewrite the strip at least this often even when the buffer has not changed,
/// so a glitched pixel heals itself.
const LED_REFRESH_US: u64 = 1_000_000; // 1s

// Simple WS2812 driver using PIO
struct Ws2812<SM: ValidStateMachine> {
    tx: Tx<SM>,
}

impl<SM: ValidStateMachine> Ws2812<SM> {
    fn write_leds(&mut self, leds: &[RGB8]) {
        for led in leds {
            // WS2812 expects GRB format, send each byte separately
            // Green byte
            while !self.tx.write((led.g as u32) << 24) {}
            // Red byte
            while !self.tx.write((led.r as u32) << 24) {}
            // Blue byte
            while !self.tx.write((led.b as u32) << 24) {}
        }
    }
}

// main loop & ! means this function never returns
//#[no_mangle no mangling (changing) of the function name
#[entry]
fn main() -> ! {
    //info!("Program start");
    // take() takes ownership and unwrap() unwraps the result, panicking if in use already
    let mut pac = pac::Peripherals::take().unwrap(); // take ownership of the peripherals (RP2040 hardware)
    let core = cortex_m::Peripherals::take().unwrap(); // take ownership of the core peripherals (Cortex-M hardware)

    // Setup watchdog to reset if the program hangs
    // Assumes program crashed if not "reset"
    let mut watchdog = Watchdog::new(pac.WATCHDOG);

    // Setup clocks and PLLs (Phase-Locked Loops)
    let clocks = init_clocks_and_plls(
        XOSC_CRYSTAL_FREQ, // 12 MHz external crystal frequency
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    ).ok().unwrap(); // ok() converts Result<Clocks, Error> to Option
    // Option is an enum that can be Some(value) or None

    // Use ARM System Timer (SYST) for delays, "how many ticks is 1 ms"
    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

    // Free-running 1MHz microsecond counter, used as the wall clock for the
    // backlight timeout and the HID tick.
    // MUST be built before the USB bus below: Timer::new borrows the whole
    // `clocks`, but UsbBus::new moves `clocks.usb_clock` out of it. Taking the
    // timer afterwards would be a borrow of a partially moved value.
    let timer = Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);

    // GPIO pin setup
    let sio = Sio::new(pac.SIO); // single-cycle IO (fastest refresh of pin states in 1 clock cycle)
    let pins = Pins::new(
        pac.IO_BANK0, // pins for config
        pac.PADS_BANK0, // rest of the pins
        sio.gpio_bank0, // SIO (RP2040 feature)
        &mut pac.RESETS, // "mutable reference to the reset controller"
    );

    // Configure the built-in LED pin as output
    let mut led_pin = pins.led.into_push_pull_output();

    // Configure all 12 keys as input
    let mut key1 = pins.key1.into_pull_up_input();
    let mut key2 = pins.key2.into_pull_up_input();
    let mut key3 = pins.key3.into_pull_up_input();
    let mut key4 = pins.key4.into_pull_up_input();
    let mut key5 = pins.key5.into_pull_up_input();
    let mut key6 = pins.key6.into_pull_up_input();
    let mut key7 = pins.key7.into_pull_up_input();
    let mut key8 = pins.key8.into_pull_up_input();
    let mut key9 = pins.key9.into_pull_up_input();
    let mut key10 = pins.key10.into_pull_up_input();
    let mut key11 = pins.key11.into_pull_up_input();
    let mut key12 = pins.key12.into_pull_up_input();

    // Initialize USB bus
    let usb_bus = UsbBusAllocator::new(UsbBus::new(
        pac.USBCTRL_REGS, //usb controller register on the rp2040
        pac.USBCTRL_DPRAM, //usb ram
        clocks.usb_clock,
        true,
        &mut pac.RESETS, 
    ));

    // Create HID devices
    let mut hid = UsbHidClassBuilder::new()
        .add_device(NKROBootKeyboardConfig::default()) // N-Key Rollover Boot Keyboard
        .add_device(ConsumerControlConfig::default()) // For media controls
        .build(&usb_bus);

    // Create USB device
    let mut usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x1209, 0x0001)) //(vendor id, product id) 0x1209 is for open-source projects
        .strings(&[StringDescriptors::default()
            .manufacturer("byanthny")
            .product("Adafruit MacroPad")
            .serial_number("00001")]).unwrap()
        .device_class(0)
        .build();

    // Initialize NeoPixels using PIO
    let (mut pio, sm0, _, _, _) = pac.PIO0.split(&mut pac.RESETS);

    // WS2812 PIO program (simplified version)
    let ws2812_program = pio_proc::pio_asm!(
        ".side_set 1"
        ".wrap_target"
        "bitloop:"
        "    out x, 1       side 0 [2]"
        "    jmp !x do_zero side 1 [1]"
        "do_one:"
        "    jmp bitloop side 1 [4]"
        "do_zero:"
        "    nop            side 0 [4]"
        ".wrap"
    );

    let installed = pio.install(&ws2812_program.program).unwrap();

    // Calculate clock divisor for WS2812 timing
    // System clock is 125MHz, we need 800kHz * 10 cycles = 8MHz for PIO
    // 125MHz / 8MHz = 15.625
    let (mut sm, _, tx) = PIOBuilder::from_installed_program(installed)
        .side_set_pin_base(pins.neopixel.id().num)
        .out_shift_direction(adafruit_macropad::hal::pio::ShiftDirection::Left)
        .autopull(true)
        .pull_threshold(8) // 8 bits per color channel
        .clock_divisor_fixed_point(15, 160) // 15.625 divisor for proper WS2812 timing
        .build(sm0);

    sm.set_pindirs([(pins.neopixel.id().num, adafruit_macropad::hal::pio::PinDir::Output)]);
    let _neopixel_pin = pins.neopixel.into_function::<FunctionPio0>();
    sm.start();

    let mut ws2812 = Ws2812 { tx };

    // Drive the strip to a known state, since WS2812 power-on state is undefined
    let mut backlight = Backlight::new(timer.get_counter().ticks());
    ws2812.write_leds(&backlight.render(timer.get_counter().ticks(), [false; 12]));

    // Wait for LEDs to latch (>50us reset time for WS2812)
    delay.delay_us(100);

    // Loop state. `last_written` starts black, so the first iteration writes the
    // strip once more than strictly needed - harmless, and cheaper than trying
    // to keep it in sync with the boot write above.
    let mut last_written: [RGB8; 12] = [RGB8 { r: 0, g: 0, b: 0 }; 12];
    let mut last_refresh_us = timer.get_counter().ticks();
    let mut last_hid_tick_us = timer.get_counter().ticks();
    let mut key12_was_pressed = false;
    let mut last_toggle_us = 0u64;

    // Turns on the LED pin when key is pressed
    loop {
        let now = timer.get_counter().ticks();

        //usb polling
        let _ = usb_dev.poll(&mut [&mut hid]);

        // hid.tick() wants to be called every 1ms. The old version counted loop
        // iterations to 1000, but an iteration is ~1.4ms (delay + USB poll +
        // a blocking 288-bit strip write), so it actually fired every ~1.4s.
        // .ok() not .unwrap(): at the correct rate a transient error is likely,
        // and panic_halt here would leave the pad dead until it is replugged.
        if now.saturating_sub(last_hid_tick_us) >= 1_000 {
            last_hid_tick_us = now;
            hid.tick().ok();
        }

        // Read every key up front. Keys 7-12 have no HID mapping but still count
        // as activity, and key 12 drives the backlight toggle.
        let pressed = [
            key1.is_low().unwrap(),
            key2.is_low().unwrap(),
            key3.is_low().unwrap(),
            key4.is_low().unwrap(),
            key5.is_low().unwrap(),
            key6.is_low().unwrap(),
            key7.is_low().unwrap(),
            key8.is_low().unwrap(),
            key9.is_low().unwrap(),
            key10.is_low().unwrap(),
            key11.is_low().unwrap(),
            key12.is_low().unwrap(),
        ];

        // The host tells us when it goes to sleep, which beats waiting out the
        // inactivity timeout on a PC that went to bed.
        backlight.set_suspended(usb_dev.state() == UsbDeviceState::Suspend, now);

        // Key 12 toggles the backlight. Rising edge only - the loop runs ~700
        // times a second, so a level check would flip it hundreds of times per
        // press - plus a lockout to swallow contact bounce.
        //
        // MUST run before note_activity(). Reversed, pressing key 12 on a dark
        // pad would have note_activity() rescue Idle -> On, and then toggle()
        // would see On and set Off, leaving the pad dark.
        if pressed[11]
            && !key12_was_pressed
            && now.saturating_sub(last_toggle_us) >= TOGGLE_LOCKOUT_US
        {
            backlight.toggle(now);
            last_toggle_us = now;
        }
        key12_was_pressed = pressed[11];

        if pressed.iter().any(|&p| p) {
            backlight.note_activity(now);
        }

        backlight.tick(now);

        // Send key reports for the mapped keys
        if pressed[0] {
            led_pin.set_high().unwrap();
            hid.device::<NKROBootKeyboard<_>, _>().write_report([Keyboard::A]).ok();
        }
        if pressed[1] {
            led_pin.set_high().unwrap();
            hid.device::<ConsumerControl<_>, _>().write_report(&MultipleConsumerReport {
                codes: [Consumer::PlayPause, Consumer::Unassigned, Consumer::Unassigned, Consumer::Unassigned]
            }).ok();
        }
        if pressed[2] {
            led_pin.set_high().unwrap();
            hid.device::<ConsumerControl<_>, _>().write_report(&MultipleConsumerReport {
                codes: [Consumer::ScanNextTrack, Consumer::Unassigned, Consumer::Unassigned, Consumer::Unassigned]
            }).ok();
        }
        if pressed[3] {
            led_pin.set_high().unwrap();
            hid.device::<NKROBootKeyboard<_>, _>().write_report([Keyboard::X]).ok();
        }
        if pressed[4] {
            led_pin.set_high().unwrap();
            hid.device::<NKROBootKeyboard<_>, _>().write_report([Keyboard::U]).ok();
        }
        if pressed[5] {
            led_pin.set_high().unwrap();
            hid.device::<NKROBootKeyboard<_>, _>().write_report([Keyboard::P]).ok();
        }

        // If no mapped key is pressed, turn off back LED and release all keys
        if !pressed[..6].iter().any(|&p| p) {
            led_pin.set_low().unwrap();
            hid.device::<NKROBootKeyboard<_>, _>().write_report([Keyboard::NoEventIndicated]).ok();
            hid.device::<ConsumerControl<_>, _>().write_report(&MultipleConsumerReport {
                codes: [Consumer::Unassigned, Consumer::Unassigned, Consumer::Unassigned, Consumer::Unassigned]
            }).ok();
        }

        // Update NeoPixel LEDs, but only when something actually changed. The
        // old code pushed 288 bits into the PIO FIFO every iteration regardless,
        // which is pure waste once the pad has gone dark.
        let leds = backlight.render(now, pressed);
        if leds != last_written || now.saturating_sub(last_refresh_us) >= LED_REFRESH_US {
            ws2812.write_leds(&leds);
            last_written = leds;
            last_refresh_us = now;
        }

        // Delay to give LEDs time to latch (needs >50μs low signal)
        delay.delay_us(1000); // 1ms delay
    }
}
