//! HIL smoke firmware: prints a marker over RTT so the host can verify that
//! flashing, reset and RTT attach all work on a physical board.
#![no_std]
#![no_main]

use cortex_m_rt::entry;
use panic_halt as _;
use rtt_target::{rprintln, rtt_init_print};

#[entry]
fn main() -> ! {
    rtt_init_print!();
    rprintln!("RAVOM-HIL-OK board=bluepill_f103c8");
    let mut ticks: u32 = 0;
    loop {
        ticks = ticks.wrapping_add(1);
        if ticks % 4_000_000 == 0 {
            rprintln!("tick {}", ticks / 4_000_000);
        }
        cortex_m::asm::nop();
    }
}
