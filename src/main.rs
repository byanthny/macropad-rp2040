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
use rp2040_hal as hal;
use embedded_hal::digital::v2::OutputPin;
use hal::{
    clocks::{init_clocks_and_plls, Clock}, //clock configurations
    pac,  // Peripheral Access Crate, "raw hardware access"
    sio::Sio,  // GPIO
    watchdog::Watchdog,
};

// Sets up the bootloader for the RP2040, 256 bytes in "boot2" section
#[unsafe(link_section = ".boot2")]
#[used] // keep this section, even if unused
pub static BOOT2: [u8; 256] = rp2040_boot2::BOOT_LOADER_GENERIC_03H; // import the bootloader from the rp2040-boot2 crate

// main loop & ! means this function never returns
//#[no_mangle no mangling (changing) of the function name
#[entry] 
fn main() -> ! {

    // take() takes ownership and unwrap() unwraps the result, panicking if in use already
    let mut pac = pac::Peripherals::take().unwrap(); // take ownership of the peripherals (RP2040 hardware)
    let core = cortex_m::Peripherals::take().unwrap(); // take ownership of the core peripherals (Cortex-M hardware)

    // Setup watchdog to reset if the program hangs
    // Assumes program crashed if not "reset"
    let mut watchdog = Watchdog::new(pac.WATCHDOG);

    // Setup clocks and PLLs (Phase-Locked Loops)
    let clocks = init_clocks_and_plls(
        12_000_000u32,  // 12MHz crystal frequency - ADD THIS LINE
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
    let pins = hal::gpio::Pins::new(
        pac.IO_BANK0, // pins for config
        pac.PADS_BANK0, // rest of the pins
        sio.gpio_bank0, // SIO (RP2040 feature)
        &mut pac.RESETS, // "mutable reference to the reset controller"
    );

    // Configure GPIO pin 1 as a push-pull output (for an LED)
    // Pushes current to the pin when high, pulls it to 0V
    let mut led_pin = pins.gpio1.into_push_pull_output();

    // Flashes the LED on and off every 500ms
    loop {
        led_pin.set_high().unwrap();  // Set High, 3.3V (LED on)
        delay.delay_ms(500);          // Wait 500ms using the delay object
        led_pin.set_low().unwrap();   // Set Low, 0V (LED off)  
        delay.delay_ms(500);    
    }
}
