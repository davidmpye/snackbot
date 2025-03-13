use defmt::*;
use libm::{log, pow };

use embassy_rp::adc::{Adc, Async, Config, InterruptHandler};
use embassy_time::{Duration, Timer};
use embassy_rp::adc;

use crate::DISPENSER_DRIVER;

const DEFAULT_TEMPERATURE_SETPOINT:f32 = 8.0;

const NUM_MEASUREMENTS_TO_AVERAGE:usize = 10;
const MEASUREMENT_INTERVAL_MS:u64 = 10;
const TEMPERATURE_MEASURE_INTERVAL_SECONDS: u64 = 60;
const CHILLER_MIN_CYCLE_COUNT: u8 = 5; //This is a multiple of the measurement interval

const THERMISTOR_PULLUP_VAL_OHMS:u64 = 10000;

const THERMISTOR_A_VAL:f64 = 2.10850817e-3;
const THERMISTOR_B_VAL:f64 = 7.97920473e-5;
const THERMISTOR_C_VAL:f64 = 6.53507631e-7;

//From: https://pico.implrust.com/thermistor/steinhart.html
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
    let inverse_temperature = a + b * ln_r + c * pow(ln_r, 3.0);//ln_r.powi(3);

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
    let mut measurements = [0u16; NUM_MEASUREMENTS_TO_AVERAGE];
    let mut pos = 0;
    let mut setpoint:f32 = DEFAULT_TEMPERATURE_SETPOINT;

    let mut chiller_change_cycle_count = 0;
    let mut chiller_current_state = false;

    loop {
        //Take specified number of measuremeents and average them.
        for val in measurements.iter_mut() {
            *val = adc.read(&mut p26).await.unwrap();
            Timer::after(Duration::from_millis(MEASUREMENT_INTERVAL_MS)).await;
        }
        let average = measurements.iter().sum::<u16>() / NUM_MEASUREMENTS_TO_AVERAGE as u16;
        
        let adc_voltage = (average as f32 / 4095.0) * 3300.0; //4095 steps in the 12 bit ADC
        let res_val =  (adc_voltage  * THERMISTOR_PULLUP_VAL_OHMS as f32) / (3300.0 - adc_voltage); //3300mV = VRef
        
        match steinhart_temp_calc(res_val as f64, THERMISTOR_A_VAL, THERMISTOR_B_VAL, THERMISTOR_C_VAL) {
            Ok(temp) => {
                info!("Drinks chiller temperature: {}'C", temp);
                if chiller_change_cycle_count == CHILLER_MIN_CYCLE_COUNT {
                    let chiller_new_state = if temp as f32 > setpoint + 0.5 {
                        true
                    }
                    else {
                        false
                    };
                    if chiller_new_state != chiller_current_state {
                        //If the desired chiler state has changed, apply it
                        let mut r = DISPENSER_DRIVER.lock().await;
                        let driver = r.as_mut().expect("Motor driver must be stored in mutex");
                        info!("Setting chiller state to {}", chiller_new_state);
                        driver.set_chiller_on(chiller_new_state).await;
                        chiller_current_state = chiller_new_state;
                    }      
                    chiller_change_cycle_count = 0;
                }
                else {
                    chiller_change_cycle_count +=1;
                }
            },
            Err(e) => {
                error!("Temperature calculation error");
            }
        }
        //Wait specified number of minutes prior to checking again.
        Timer::after(Duration::from_secs(TEMPERATURE_MEASURE_INTERVAL_SECONDS)).await;
    }
}
