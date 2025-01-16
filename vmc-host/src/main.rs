mod lcd_driver;
use lcd_driver::LcdDriver;

mod vmc_driver;
use vmc_driver::VmcDriver;


#[tokio::main]
async fn main() {
    println!("VMC Host initialising");
    match LcdDriver::new("keyboard") {
        Ok(mut driver) => {
            let _ = driver.set_text(String::from("GOOD MORNING"), String::from("VIETNAM")).await;
        },
        Err(msg) => {
            println!("Unable to connect to Keyboard LCD display driver:  {}", msg);
        }
    }

    match VmcDriver::new("vmc") {
        Ok(mut driver) => {
            println!("Connected to VMC - mapping");
            println!("{:?}", driver.map_machine().await);
        },
        Err(msg) => {
            println!("Unable to connect to VMC:  {}", msg);
        }
    }
} 
