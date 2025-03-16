use embassy_rp::watchdog::Watchdog;
use embassy_rp::peripherals::WATCHDOG;
use embassy_time::{Duration, Timer};

const WATCHDOG_TIMER_SECS:u64 = 2;
const WATCHDOG_FEED_TIMER_MS:u64 = 250;

#[embassy_executor::task]
pub async fn watchdog_task(watchdog: WATCHDOG) -> ! {

    let mut dog = Watchdog::new(watchdog);

    dog.start(Duration::from_secs(WATCHDOG_TIMER_SECS));
    
    loop {
        Timer::after(Duration::from_millis(WATCHDOG_FEED_TIMER_MS)).await;
        dog.feed();
    }
}