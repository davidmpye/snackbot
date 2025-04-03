use embassy_rp::adc::{Adc, Async};
use embassy_time::{Duration, Timer};
use embassy_rp::adc;

use embassy_rp::gpio::{Level, Output};

use defmt::*;
use libm::{log, pow};

use crate::DISPENSER_DRIVER;

const DEFAULT_TEMPERATURE_SETPOINT:f32 = 8.0;

const NUM_MEASUREMENTS_TO_AVERAGE:usize = 10;
const MEASUREMENT_DELAY:Duration = Duration::from_millis(10);
const TEMPERATURE_MEASURE_INTERVAL:Duration = Duration::from_secs(60);
const CHILLER_MIN_CYCLE_COUNT: u8 = 5; //This is a multiple of the measurement interval

const THERMISTOR_PULLUP_VAL_OHMS:u64 = 10000;

//For a 2.2k thermistor (https://www.bapihvac.com/wp-content/uploads/2010/11/Thermistor_2.2K.pdf),
//calculated using https://rusefi.com/Steinhart-Hart.html
//const THERMISTOR_A_VAL:f64 = 1.4726620300667711e-3;
//const THERMISTOR_B_VAL:f64 = 2.3739290559817496e-4;
//const THERMISTOR_C_VAL:f64 = 1.060205944258554e-7;

//3.3k:
const THERMISTOR_A_VAL:f64 = 1.3811057615602958e-3;
const THERMISTOR_B_VAL:f64 = 2.370102475713365e-4;
const THERMISTOR_C_VAL:f64 = 9.879312896211082e-8;


#[embassy_executor::task]
pub async fn chiller_task(
    mut adc: Adc<'static, Async>,
    mut channel: adc::Channel<'static>,
    mut led_pin: Output<'static>,
) -> ! {
    let mut measurements = [0u16; NUM_MEASUREMENTS_TO_AVERAGE];
    //Fixme - add channel to allow setpoint to be changed
    let setpoint:f32 = DEFAULT_TEMPERATURE_SETPOINT;

    let mut chiller_change_cycle_count = CHILLER_MIN_CYCLE_COUNT; //this forces initial compute
    let mut chiller_current_state = false;

    loop {
        //Take specified number of measurements and average them.c
        for val in measurements.iter_mut() {
            *val = adc.read(&mut channel).await.unwrap();
            Timer::after(MEASUREMENT_DELAY).await;
        }
        let average = measurements.iter().sum::<u16>() / NUM_MEASUREMENTS_TO_AVERAGE as u16;
        
        let adc_voltage = (average as f32 / 4095.0) * 3300.0; //4095 steps in the 12 bit ADC
        let res_val =  (adc_voltage  * THERMISTOR_PULLUP_VAL_OHMS as f32) / (3300.0 - adc_voltage); //3300mV = VRef
        
        match steinhart_temp_calc(res_val as f64, THERMISTOR_A_VAL, THERMISTOR_B_VAL, THERMISTOR_C_VAL) {
            Ok(temp) => {
                //We only turn on/off the chiller every MIN_CYCLE_COUNT poll intervals as it won't like
                //being repeatedly turned on/off.
                if chiller_change_cycle_count == CHILLER_MIN_CYCLE_COUNT {
                    let chiller_new_state=  temp as f32 > setpoint + 0.5;
                    if chiller_new_state != chiller_current_state {
                        //If the desired chiler state has changed, apply it
                        let mut r = DISPENSER_DRIVER.lock().await;
                        let driver = r.as_mut().expect("Motor driver must be stored in mutex");
                        debug!("Setting chiller state to {}", chiller_new_state);
                        driver.set_chiller_on(chiller_new_state).await;
                        chiller_current_state = chiller_new_state;

                        //Set the board-mounted status LED to
                        let led_level = if chiller_current_state {
                            Level::High
                        }
                        else {
                            Level::Low
                        };
                        led_pin.set_level(led_level);
                    }      
                    chiller_change_cycle_count = 0;
                }
                else {
                    chiller_change_cycle_count +=1;
                }
                info!("Drinks chiller temperature: {}'C, target {}'C, chiller_on: {}", temp, setpoint, chiller_current_state);
            },
            Err(_e) => {
                error!("Steinhart-Hart temperature calculation error");
            }
        }
        //Wait specified period prior to checking again.
        Timer::after(TEMPERATURE_MEASURE_INTERVAL).await;
    }
}

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