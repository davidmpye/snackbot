mod lcd_driver;
use lcd_driver::LcdDriver;

mod vmc_driver;
use vmc_driver::VmcDriver;

const KEYBOARD_DEVICE_NAME:&str = "keyboard";
const VMC_DEVICE_NAME:&str = "vmc";

#[tokio::main]
async fn main() {
    println!("VMC Host initialising");
    match LcdDriver::new(KEYBOARD_DEVICE_NAME) {
        Ok(mut driver) => {
            let _ = driver.set_text(String::from("Snackbot"), String::from("Makerspace Vending Solutions")).await;
        },
        Err(msg) => {
            println!("Unable to connect to Keyboard LCD display driver:  {}", msg);
        }
    }

    match VmcDriver::new(VMC_DEVICE_NAME) {
        Ok(mut driver) => {
            println!("Connected to VMC - mapping machine");
            println!("{:?}", driver.map_machine().await);
        },
        Err(msg) => {
            println!("Unable to connect to VMC:  {}", msg);
        }
    }
} 
