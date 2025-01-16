mod lcd_driver;

use keyboard_icd;
use lcd_driver::LcdDriver;

#[tokio::main]
async fn main() {
    println!("VMC Host initialising");

    match LcdDriver::new() {
        Ok(mut driver) => {
            let _ = driver.set_text(String::from("GOOD MORNING"), String::from("VIETNAM")).await;
        },
        Err(msg) => {
            println!("Unable to connect to Keyboard LCD display driver component:  {}", msg);
        }
    }

    



} 
