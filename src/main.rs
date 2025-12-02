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
        usb::UsbBus,
        pio::PIOExt,
    },
    Pins, XOSC_CRYSTAL_FREQ,
};

// USB HID imports
use usb_device::{prelude::*, class_prelude::*};
use usbd_human_interface_device::{prelude::*, page::{Keyboard, Consumer}, device::{keyboard::{NKROBootKeyboardConfig, NKROBootKeyboard}, consumer::{ConsumerControlConfig, ConsumerControl, MultipleConsumerReport}}}; //MultipleConsumerReport has 4 consumer control codes

// NeoPixel LED imports
use smart_leds::RGB8;
use adafruit_macropad::hal::pio::{PIOBuilder, Tx, ValidStateMachine};
use adafruit_macropad::hal::gpio::FunctionPio0;

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

    // Define colors
    let warm_white = RGB8::new(255, 200, 120); // Warm white color
    let key_press_color = RGB8::new(0, 255, 100); // Cyan/green for key press

    // Initialize LED array - all LEDs start as warm white
    let mut leds = [warm_white; 12];

    // Set initial LED state
    ws2812.write_leds(&leds);

    // Wait for LEDs to latch (>50us reset time for WS2812)
    delay.delay_us(100);

    let mut last_tick = 0u32; //set unsigned 32-bit integer for last tick

    // Turns on the LED pin when key is pressed
    loop {

        //usb polling
        let _ = usb_dev.poll(&mut [&mut hid]);

        //tick about every 1ms? check math, idle rate compliance -> look into HID compliance more
        last_tick += 1;
        if last_tick >= 1000 {
            last_tick = 0;
            hid.tick().unwrap();
        }

        // Read key states
        let key1_pressed = key1.is_low().unwrap();
        let key2_pressed = key2.is_low().unwrap();
        let key3_pressed = key3.is_low().unwrap();
        let key4_pressed = key4.is_low().unwrap();
        let key5_pressed = key5.is_low().unwrap();
        let key6_pressed = key6.is_low().unwrap();
        let key7_pressed = key7.is_low().unwrap();
        let key8_pressed = key8.is_low().unwrap();
        let key9_pressed = key9.is_low().unwrap();
        let key10_pressed = key10.is_low().unwrap();
        let key11_pressed = key11.is_low().unwrap();
        let key12_pressed = key12.is_low().unwrap();

        // Reset all LEDs to warm white
        leds = [warm_white; 12];

        // Update LED colors for pressed keys and send key reports
        if key1_pressed {
            led_pin.set_high().unwrap();
            leds[0] = key_press_color;
            hid.device::<NKROBootKeyboard<_>, _>().write_report([Keyboard::A]).ok();
        }
        if key2_pressed {
            led_pin.set_high().unwrap();
            leds[1] = key_press_color;
            hid.device::<ConsumerControl<_>, _>().write_report(&MultipleConsumerReport {
                codes: [Consumer::PlayPause, Consumer::Unassigned, Consumer::Unassigned, Consumer::Unassigned]
            }).ok();
        }
        if key3_pressed {
            led_pin.set_high().unwrap();
            leds[2] = key_press_color;
            hid.device::<ConsumerControl<_>, _>().write_report(&MultipleConsumerReport {
                codes: [Consumer::ScanNextTrack, Consumer::Unassigned, Consumer::Unassigned, Consumer::Unassigned]
            }).ok();
        }
        if key4_pressed {
            led_pin.set_high().unwrap();
            leds[3] = key_press_color;
            hid.device::<NKROBootKeyboard<_>, _>().write_report([Keyboard::X]).ok();
        }
        if key5_pressed {
            led_pin.set_high().unwrap();
            leds[4] = key_press_color;
            hid.device::<NKROBootKeyboard<_>, _>().write_report([Keyboard::U]).ok();
        }
        if key6_pressed {
            led_pin.set_high().unwrap();
            leds[5] = key_press_color;
            hid.device::<NKROBootKeyboard<_>, _>().write_report([Keyboard::P]).ok();
        }

        // If no keys pressed, turn off back LED and release all keys
        if !key1_pressed && !key2_pressed && !key3_pressed && !key4_pressed && !key5_pressed && !key6_pressed {
            led_pin.set_low().unwrap();
            hid.device::<NKROBootKeyboard<_>, _>().write_report([Keyboard::NoEventIndicated]).ok();
            hid.device::<ConsumerControl<_>, _>().write_report(&MultipleConsumerReport {
                codes: [Consumer::Unassigned, Consumer::Unassigned, Consumer::Unassigned, Consumer::Unassigned]
            }).ok();
        }

        // Update NeoPixel LEDs
        ws2812.write_leds(&leds);

        // Delay to give LEDs time to latch (needs >50μs low signal)
        delay.delay_us(1000); // 1ms delay
    }
}


