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

    loop {
        //forever
    }
}
