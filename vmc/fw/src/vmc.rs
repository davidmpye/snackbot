use embassy_time::{Duration, Timer};
use postcard_rpc::server::{impls::embassy_usb_v0_4::EUsbWireTx, Sender};
use embassy_rp::peripherals::USB;
use embassy_rp::usb::Driver as UsbDriver;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;

#[embassy_executor::task]
pub async fn vmc_task(
    postcard_sender: Sender<EUsbWireTx<ThreadModeRawMutex, UsbDriver<'static, USB>>>,
) -> ! {
    loop {

    const CASHLESS_DEVICE_POLL_INTERVAL: Duration = Duration::from_millis(100);
    Timer::after(CASHLESS_DEVICE_POLL_INTERVAL).await;
    }
}