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
use embedded_hal::digital::OutputPin;
use adafruit_macropad::{
    hal::{
        clocks::{init_clocks_and_plls, Clock},
        pac,
        watchdog::Watchdog,
        Sio,
    },
    Pins, XOSC_CRYSTAL_FREQ,
};

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

    // Flashes the LED on and off every 500ms
    loop {
        led_pin.set_high().unwrap();
        delay.delay_ms(500);
        led_pin.set_low().unwrap();
        delay.delay_ms(500);
    }
}
