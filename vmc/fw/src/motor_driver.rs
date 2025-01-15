use defmt::*;

use embassy_time::{Duration, Timer, WithTimeout};

use embassy_rp::gpio::{AnyPin, Level, OutputOpenDrain};

use vmc_icd::{
    CanStatus, Dispense, DispenseError, DispenseResult, Dispenser, DispenserAddress, DispenserOption, DispenserType, ForceDispense, 
    MotorStatus,GetDispenserInfo, ENDPOINT_LIST, TOPICS_IN_LIST, TOPICS_OUT_LIST
};

pub struct MotorDriver<'a> {
    bus: [OutputOpenDrain<'a>; 8],
    clks: [OutputOpenDrain<'a>; 3],
    output_enable: OutputOpenDrain<'a>,
    flipflop_clr: OutputOpenDrain<'a>,
}

impl<'a> MotorDriver<'a> {
    pub(crate) async fn new(
        bus_pin0: AnyPin,
        bus_pin1: AnyPin,
        bus_pin2: AnyPin,
        bus_pin3: AnyPin,
        bus_pin4: AnyPin,
        bus_pin5: AnyPin,
        bus_pin6: AnyPin,
        bus_pin7: AnyPin,

        clk_pin1: AnyPin,
        clk_pin2: AnyPin,
        clk_pin3: AnyPin,

        oe_pin: AnyPin,
        flipflop_clr_pin: AnyPin,
    ) -> Self {
        let mut x = Self {
            bus: [
                OutputOpenDrain::new(bus_pin0, Level::High),
                OutputOpenDrain::new(bus_pin1, Level::High),
                OutputOpenDrain::new(bus_pin2, Level::High),
                OutputOpenDrain::new(bus_pin3, Level::High),
                OutputOpenDrain::new(bus_pin4, Level::High),
                OutputOpenDrain::new(bus_pin5, Level::High),
                OutputOpenDrain::new(bus_pin6, Level::High),
                OutputOpenDrain::new(bus_pin7, Level::High),
            ],
            clks: [
                OutputOpenDrain::new(clk_pin1, Level::High),
                OutputOpenDrain::new(clk_pin2, Level::High),
                OutputOpenDrain::new(clk_pin3, Level::High),
            ],
            output_enable: OutputOpenDrain::new(oe_pin, Level::High),
            flipflop_clr: OutputOpenDrain::new(flipflop_clr_pin, Level::High),
        };

        //Pull flipflop_clr high after 50uS to allow flipflops to be written
        Timer::after_micros(50).await;
        x.flipflop_clr.set_low();
        Timer::after_micros(50).await;
        x.flipflop_clr.set_high();

        x
    }

    async fn stop_motors(&mut self) {
        //Stop all motors
        debug!("Stopped all motors");
        self.write_bytes([0x00,0x00,0x00]).await;
    }

    async fn drive_motor(&mut self, row: char, col: char) {
        debug!("Driving motor at {}{}", row, col);
        let bytes = MotorDriver::calc_drive_bytes(row, col).unwrap();
        self.write_bytes(bytes).await;
    }

    async fn write_bytes(&mut self, bytes: [u8; 3]) {
        debug!("Writing out bytes {}", bytes);
        for (clk_pin, byte) in core::iter::zip(self.clks.iter_mut(), bytes.iter()) {
            //write out the data
            for (bit_index, gpio) in self.bus.iter_mut().enumerate() {
                if byte & (0x01 << bit_index) == 0 {
                    gpio.set_low();
                } else {
                    gpio.set_high();
                }
            }
            clk_pin.set_low();
            Timer::after_micros(1).await;
            clk_pin.set_high();
            Timer::after_micros(1).await;
        }

        //Release the pins
        for gpio in self.bus.iter_mut() {
            gpio.set_high();
        }
    }

    fn calc_drive_bytes(row: char, col: char) -> Result<[u8; 3], ()> {
        /*
        Wiring is as follows

        U2:                     U3:
        ===============         ===============
        0x01 - Row E Even       0x01 - Cols 0,1
        0x02 - Row E Odd        0x02 - Cols 2,3
        0x04 - Row F Even       0x04 - Cols 4,5
        0x08 - Row F Odd        0x08 - Cols 6,7
                                0x10 - Cols 8,9
                                0x20 - Row G (there's no odd!) - Gum and mint row drive (if fitted)
        U4:
        ===============
        0x01 - Row A Even
        0x02 - Row A Odd
        0x04 - Row B Even
        0x08 - Row B Odd
        0x10 - Row C Even
        0x20 - Row C Odd
        0x40 - Row D Even
        0x80 - Row D Odd
        */
        let mut drive_bytes: [u8; 3] = [0x00; 3];

        //Check row and col are calculable - note, NOT whether they are present in the machine
        if !row.is_ascii_uppercase() || row < 'A' || row > 'G' {
            return Err(());
        }

        if !col.is_ascii_digit() {
            return Err(());
        }

        let row_offset = row as u8 - b'A';
        let col_offset = match row as u8 {
            //Special handling for can chiller rows (E/F) due to discrepancy in numbering and wiring!
            //Row E +F cans are numbered E0, E1, E2, E3 but are wired E0, E2, E4, E6
            //G - Gum and Mint may need special handling if implemented as I suspect that's wired 0/2/4/6/8 also.
            //G is the optional Gum/Mint module.
            b'E' | b'F' => (col as u8 - b'0') * 2,
            //Standard column offset
            _ => col as u8 - b'0',
        };

        let even_odd_offset: u8 = col_offset % 2;

        //Set row drive bit on appropriate flipflop
        match row as u8 {
            b'A' | b'B' | b'C' | b'D' => {
                //U4
                drive_bytes[2] = 0x01 << (row_offset * 2 + even_odd_offset);
            }
            b'E' | b'F' => {
                //U2
                drive_bytes[0] = 0x01 << ((row_offset - 4) * 2 + even_odd_offset);
            }
            b'G' => {
                //U3
                drive_bytes[1] = 0x20;
            }
            _ => {
                //This shouldn't happen!
                defmt::panic!("Asked to apply invalid row calculation!")
            }
        }
        //Set column drive bit
        drive_bytes[1] |= 0x01 << (col_offset / 2);

        debug!(
            "Calculated drive byte for {}{} as {=[u8]:#04x}",
            row, col, drive_bytes
        );
        Ok(drive_bytes)
    }

    fn motor_homed_gpio_index(row: char, col: char) -> usize {
        //Helper function to give us the bus gpio index to see motor home status
        match row {
            'E' => 4, //0x10u8
            'F' => 6, //0x40u8,
            _ => {
                if col.to_digit(10).unwrap_or(0) % 2 == 0 {
                    0 //0x01
                } else {
                    1 //0x02u8
                }
            }
        }
    }

    pub async fn pulse_motor_and_read_gpio(&mut self, row: char, col:char , gpio_index: usize) -> bool {
        //Power motor
        self.drive_motor(row,col).await;
        //Buffer to READ mode
        self.output_enable.set_low();
        Timer::after_micros(20).await;
        let state = self.bus[gpio_index].is_high();
        self.output_enable.set_high();
        Timer::after_micros(20).await;
        self.stop_motors().await;
        state
    }

    //Is the motor home at this precise moment in time?
    pub async fn is_home(&mut self, row: char, col: char) -> bool {
        let is_home = self.pulse_motor_and_read_gpio(row, col, MotorDriver::motor_homed_gpio_index(row, col)).await;
        debug!("Checked to see if {}{} is home - {}", row, col, is_home);
        is_home
    }

    async fn can_status(&mut self, row: char, col: char) -> Option<CanStatus> {
        //Our can rows are E and F
        if row != 'E' && row != 'F' {
            debug!("Checked can status for non can row {}", row);
            return None;
        }
        let can_status_gpio:usize = match row {
            'E' => 5,
            'F' => 7,
             _=> 0,
        };
        debug!("Checking can status for {}{}, GPIO {}", row, col, can_status_gpio);

        let status = if self.pulse_motor_and_read_gpio(row, col, can_status_gpio).await {
            debug!("Has cans");
            CanStatus::Ok
        }
        else {
            debug!("Last can");
            CanStatus::LastCan
        };
        Some(status)
    }

    pub async fn dispense(&mut self, row: char, col: char) -> DispenseResult {
        if ! self.is_home(row, col).await {
            error!("Refusing to dispense item - motor not home at start of vend");
            return Err(DispenseError::MotorNotHome);
        }
        self.force_dispense(row, col).await
    }

    pub async fn force_dispense (&mut self, row: char, col: char) -> DispenseResult {
        debug!("Driving dispense motor at {}{}", row, col);
        self.drive_motor(row,col).await;
        let home_gpio_index = MotorDriver::motor_homed_gpio_index(row, col);
        debug!("Motor homed gpio index is is {}", home_gpio_index);
        debug!("Waiting for motor to leave home");

        self.output_enable.set_low();
        //Buffer seems to need time to 'settle'
        Timer::after_micros(20).await;

        let b = self.bus[home_gpio_index]
            .wait_for_falling_edge()
            .with_timeout(Duration::from_millis(1500))
            .await;

        if b.is_ok() {
            debug!("Motor left home");
        } else {
            error!("Motor did not leave home in time (1 sec)");
            //Turn the buffer off again.
            self.output_enable.set_high();
            Timer::after_micros(20).await;
            self.stop_motors().await;
            return Err(DispenseError::MotorStuckHome);
        }

            //Now the motor is moving, it has 3 seconds to return home to complete the vend cycles
        let b = self.bus[home_gpio_index]
            .wait_for_rising_edge()
            .with_timeout(Duration::from_millis(3500))
            .await;

        //Buffer off.
        self.output_enable.set_high();
        Timer::after_micros(20).await;

        //Motor off
        self.stop_motors().await;

        if b.is_ok() {
            info!("Vend completed successfully");
            Ok(())
        } else {
            error!("Motor did not return home in time (3 sec)");
            return Err(DispenseError::MotorStuckNotHome);
        }
    }
}
