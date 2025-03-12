use defmt::*;

use crate::DISPENSER_DRIVER;

use libm::{log, pow };
use embassy_rp::adc::{Adc, Async, Config, InterruptHandler};
use embassy_time::{Duration, Timer};
use embassy_rp::adc;

fn steinhart_temp_calc(
    resistance: f64, // Resistance in Ohms
    a: f64,          // Coefficient A
    b: f64,          // Coefficient B
    c: f64,          // Coefficient C
) -> Result<f64, ()> {
    if resistance <= 0.0 {
        return Err(());
    }
    // Calculate temperature in Kelvin using Steinhart-Hart equation:
    // 1/T = A + B*ln(R) + C*(ln(R))^3
    let ln_r = log(resistance);
    let inverse_temperature = a + b * ln_r + c * pow(ln_r, 3 as  f64);//ln_r.powi(3);

    if inverse_temperature == 0.0 {       
         return Err(());
    }

    let temperature_kelvin = 1.0 / inverse_temperature;
    let temperature_celsius = temperature_kelvin - 273.15;

    Ok(temperature_celsius)
}


#[embassy_executor::task]
pub async fn chiller_task(
    mut adc: Adc<'static, Async>,
    mut p26: adc::Channel<'static>,
) -> ! {
    let mut measurements = [0u16; 10];
    let mut pos = 0;
    let mut setpoint:f32 = 15.0;

    loop {
        //Take 10 temperature measurements, each 100mS apart
        for i in 0..10 {
            measurements[i] = adc.read(&mut p26).await.unwrap();
            Timer::after(Duration::from_millis(100)).await;
        }
        //Average them
        let average = measurements.iter().sum::<u16>() / 10;
        
        //Our thermistor is a 10k thermistor, pulled up by 10k.
        let adc_voltage = (average as f32 / 4095 as f32) * 3300 as f32;
        let res_val =  (adc_voltage  * 10000 as f32) / (3300 as f32 - adc_voltage)  as f32;

        let a = 2.10850817e-3;
        let b = 7.97920473e-5;
        let c = 6.53507631e-7;
        
        match steinhart_temp_calc(res_val as f64, a, b, c) {
            Ok(temp) => {
                info!("Temperature in Celsius: {}", temp);
                let chiller_state = if temp as f32 > setpoint + 0.5 as f32 {
                    true
                }
                else {
                    false
                };

                {
                    let mut r = DISPENSER_DRIVER.lock().await;
                    let driver = r.as_mut().expect("Motor driver must be stored in mutex");
                    driver.set_chiller_on(chiller_state).await
                }

                //Wait 10 mins prior to checking again.
                Timer::after(Duration::from_secs(60 * 10)).await;
            }
            Err(e) => {
                error!("Temperature calculation error");
            }
        }
    }
}
