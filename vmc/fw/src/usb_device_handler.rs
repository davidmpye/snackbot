use core::sync::atomic::{AtomicBool, Ordering};
use defmt::*;
use embassy_rp::usb::Driver as UsbDriver;
use embassy_usb::{Handler, UsbDevice};
use embassy_rp::peripherals::{USB};
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
    fn enabled(&mut self, enabled: bool) {
        self.configured.store(false, Ordering::Relaxed);
        if enabled {
            debug!("USB device enabled");
        } else {
            debug!("USB device disabled");
        }
    }

    fn reset(&mut self) {
        self.configured.store(false, Ordering::Relaxed);
        debug!("USB reset, the Vbus current limit is 100mA");
    }

    fn addressed(&mut self, addr: u8) {
        self.configured.store(false, Ordering::Relaxed);
        debug!("USB address set to: {}", addr);
    }

    fn configured(&mut self, configured: bool) {
        self.configured.store(configured, Ordering::Relaxed);
        if configured {
            debug!(
                "USB device configured, it may now draw up to the configured current limit from Vbus."
            )
        } else {
            debug!("USB device is no longer configured, the Vbus current limit is 100mA.");
        }
    }
}
