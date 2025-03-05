use core::sync::atomic::{AtomicBool, Ordering};
use embassy_rp::usb::Driver as UsbDriver;
use embassy_usb::{Handler, UsbDevice};
use embassy_rp::peripherals::USB;

type MyUsbDriver = UsbDriver<'static, USB>;
type MyUsbDevice = UsbDevice<'static, MyUsbDriver>;

#[embassy_executor::task]
pub (crate) async fn usb_task(mut usb: MyUsbDevice) -> ! {
    usb.run().await
}

pub (crate)struct UsbDeviceHandler {
    configured: AtomicBool,
}

impl UsbDeviceHandler {
    pub(crate) fn new() -> Self {
        UsbDeviceHandler {
            configured: AtomicBool::new(false),
        }
    }
}

impl Handler for UsbDeviceHandler {
    fn enabled(&mut self, _enabled: bool) {
        self.configured.store(false, Ordering::Relaxed);
    }

    fn reset(&mut self) {
        self.configured.store(false, Ordering::Relaxed);
    }

    fn addressed(&mut self, _addr: u8) {
        self.configured.store(false, Ordering::Relaxed);
    }

    fn configured(&mut self, configured: bool) {
        self.configured.store(configured, Ordering::Relaxed);
    }
}
