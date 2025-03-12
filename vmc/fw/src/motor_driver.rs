use defmt::*;

use embassy_rp::gpio::{Level, OutputOpenDrain};
use embassy_time::{Duration, Timer, WithTimeout};

use vmc_icd::dispenser::{
    CanStatus, DispenseError, DispenseResult, Dispenser, DispenserAddress, DispenseCommand,
    DispenserOption, DispenserType, MotorStatus,
};
use vmc_icd::DispenseEndpoint;

use postcard_rpc::header::VarHeader;

use crate::{AppTx, MotorDriverResources, Sender, SpawnCtx, Context, MOTOR_DRIVER};

#[embassy_executor::task]
pub async fn motor_driver_dispense_task(
    _context: SpawnCtx,
    header: VarHeader,
    rqst: DispenseCommand,
    sender: Sender<AppTx>,
) {
    let mut r = MOTOR_DRIVER.lock().await;
    let driver = r.as_mut().expect("Motor driver must be stored in mutex");
    let result = match rqst {
        DispenseCommand::Vend(addr) => {
            info!("Attempted vend");
            driver.dispense(addr).await
        }
        DispenseCommand::ForceVend(addr) => {
            info!("Attempted forcevend");
            driver.force_dispense(addr).await
        }
    };
    let _ = sender.reply::<DispenseEndpoint>(header.seq_no, &result).await;
}

pub async fn motor_driver_dispenser_status(    
    context: &mut Context,
    _header: VarHeader,
    addr: DispenserAddress) -> Option<Dispenser> {
    let mut r = MOTOR_DRIVER.lock().await;
    let driver = r.as_mut().expect("Motor driver must be stored in mutex");
    driver.get_dispenser(addr).await
}

pub struct MotorDriver<'a> {
    bus: [OutputOpenDrain<'a>; 8],
    clks: [OutputOpenDrain<'a>; 3],
    output_enable: OutputOpenDrain<'a>,
    flipflop_clr: OutputOpenDrain<'a>,
    valid_addresses: [DispenserAddress; 24],
    chiller_on: bool,
}

impl<'a> MotorDriver<'a> {
    pub(crate) async fn new(pins: MotorDriverResources) -> Self {
        let mut x = Self {
            bus: [
                OutputOpenDrain::new(pins.p0, Level::High),
                OutputOpenDrain::new(pins.p1, Level::High),
                OutputOpenDrain::new(pins.p2, Level::High),
                OutputOpenDrain::new(pins.p3, Level::High),
                OutputOpenDrain::new(pins.p4, Level::High),
                OutputOpenDrain::new(pins.p5, Level::High),
                OutputOpenDrain::new(pins.p6, Level::High),
                OutputOpenDrain::new(pins.p7, Level::High),
            ],
            clks: [
                OutputOpenDrain::new(pins.clk0, Level::High),
                OutputOpenDrain::new(pins.clk1, Level::High),
                OutputOpenDrain::new(pins.clk2, Level::High),
            ],
            output_enable: OutputOpenDrain::new(pins.oe, Level::High),
            flipflop_clr: OutputOpenDrain::new(pins.clr, Level::High),

            valid_addresses: [
                DispenserAddress { row: 'A', col: '0' },
                DispenserAddress { row: 'A', col: '2' },
                DispenserAddress { row: 'A', col: '4' },
                DispenserAddress { row: 'A', col: '6' },
                DispenserAddress { row: 'B', col: '0' },
                DispenserAddress { row: 'B', col: '2' },
                DispenserAddress { row: 'B', col: '4' },
                DispenserAddress { row: 'B', col: '6' },
                DispenserAddress { row: 'C', col: '0' },
                DispenserAddress { row: 'C', col: '1' },
                DispenserAddress { row: 'C', col: '2' },
                DispenserAddress { row: 'C', col: '3' },
                DispenserAddress { row: 'C', col: '4' },
                DispenserAddress { row: 'C', col: '5' },
                DispenserAddress { row: 'C', col: '6' },
                DispenserAddress { row: 'C', col: '7' },
                //Our cans
                DispenserAddress { row: 'E', col: '0' },
                DispenserAddress { row: 'E', col: '1' },
                DispenserAddress { row: 'E', col: '2' },
                DispenserAddress { row: 'E', col: '3' },
                DispenserAddress { row: 'F', col: '0' },
                DispenserAddress { row: 'F', col: '1' },
                DispenserAddress { row: 'F', col: '2' },
                DispenserAddress { row: 'F', col: '3' },
            ],
            chiller_on: false,
        };

        //Clear the flipflops using flipflop_clr
        Timer::after_micros(50).await;
        x.flipflop_clr.set_low();
        Timer::after_micros(50).await;
        x.flipflop_clr.set_high();

        x
    }

    fn is_address_valid(&mut self, addr: DispenserAddress) -> bool {
        self.valid_addresses.contains(&addr)
    }

    async fn stop_motors(&mut self) {
        //Stop all motors
        debug!("Stopped all motors");
        self.write_bytes([0x00, 0x00, 0x00]).await;
    }

    async fn drive_motor(&mut self, addr: DispenserAddress) {
        debug!("Driving motor at {}{}", addr.row, addr.col);
        let bytes = MotorDriver::calc_drive_bytes(addr).unwrap();
        self.write_bytes(bytes).await;
    }

    async fn write_bytes(&mut self, mut bytes: [u8; 3]) {
        //Add in the chiller state
        if self.chiller_on {
            bytes[0] |= 0x10u8;
        }
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

    fn calc_drive_bytes(addr: DispenserAddress) -> Result<[u8; 3], ()> {
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
        if !addr.row.is_ascii_uppercase() || addr.row < 'A' || addr.row > 'G' {
            return Err(());
        }

        if !addr.col.is_ascii_digit() {
            return Err(());
        }

        let row_offset = addr.row as u8 - b'A';
        let col_offset = match addr.row as u8 {
            //Special handling for can chiller rows (E/F) due to discrepancy in numbering and wiring!
            //Row E +F cans are numbered E0, E1, E2, E3 but are wired E0, E2, E4, E6
            //G - Gum and Mint may need special handling if implemented as I suspect that's wired 0/2/4/6/8 also.
            //G is the optional Gum/Mint module.
            b'E' | b'F' => (addr.col as u8 - b'0') * 2,
            //Standard column offset
            _ => addr.col as u8 - b'0',
        };

        let even_odd_offset: u8 = col_offset % 2;

        //Set row drive bit on appropriate flipflop
        match addr.row as u8 {
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
            addr.row, addr.col, drive_bytes
        );
        Ok(drive_bytes)
    }

    fn motor_homed_gpio_index(addr: DispenserAddress) -> usize {
        //Helper function to give us the bus gpio index to see motor home status
        match addr.row {
            'E' => 4, //0x10u8
            'F' => 6, //0x40u8,
            _ => {
                if addr.col.to_digit(10).unwrap_or(0) % 2 == 0 {
                    0 //0x01
                } else {
                    1 //0x02u8
                }
            }
        }
    }

    async fn pulse_motor_and_read_gpio(
        &mut self,
        addr: DispenserAddress,
        gpio_index: usize,
    ) -> Level {
        //Power motor
        self.drive_motor(addr).await;
        //Buffer to READ mode
        self.output_enable.set_low();
        Timer::after_micros(20).await;
        let state = self.bus[gpio_index].get_level();
        self.output_enable.set_high();
        Timer::after_micros(20).await;
        self.stop_motors().await;
        state
    }

    pub async fn get_dispenser(&mut self, addr: DispenserAddress) -> DispenserOption {
        if self.is_address_valid(addr) {
            Some(Dispenser {
                address: addr,
                dispenser_type: match addr.row {
                    'E' | 'F' => DispenserType::Can,
                    _ => DispenserType::Spiral,
                },
                motor_status: self.motor_home_status(addr).await,
                can_status: self.can_status(addr).await,
            })
        } else {
            None
        }
    }

    async fn motor_home_status(&mut self, addr: DispenserAddress) -> MotorStatus {
        let is_home = self
            .pulse_motor_and_read_gpio(addr, MotorDriver::motor_homed_gpio_index(addr))
            .await;
        debug!(
            "Checked to see if {}{} is home - {}",
            addr.row,
            addr.col,
            is_home == Level::High
        );

        if is_home == Level::High {
            MotorStatus::Ok
        } else {
            MotorStatus::MotorNotHome
        }
    }

    async fn can_status(&mut self, addr: DispenserAddress) -> Option<CanStatus> {
        //Our can rows are E and F
        if addr.row != 'E' && addr.row != 'F' {
            debug!("Checked can status for non can row {}", addr.row);
            return None;
        }

        let can_status_gpio: usize = match addr.row {
            'E' => 5,
            'F' => 7,
            _ => 0,
        };
        debug!(
            "Checking can status for {}{}, GPIO {}",
            addr.row, addr.col, can_status_gpio
        );

        let status = if self.pulse_motor_and_read_gpio(addr, can_status_gpio).await == Level::High {
            debug!("Has cans");
            CanStatus::Ok
        } else {
            debug!("Last can");
            CanStatus::LastCan
        };
        Some(status)
    }

    pub async fn dispense(&mut self, addr: DispenserAddress) -> DispenseResult {
        if self.motor_home_status(addr).await != MotorStatus::Ok {
            error!("Refusing to dispense item - motor not home at start of vend");
            return Err(DispenseError::MotorNotHome);
        }
        //If it is a can row, check if we have >1 can left too
        if let Some(can_state) = self.can_status(addr).await {
            if can_state != CanStatus::Ok {
                error!("Refusing to dispense last can");
                return Err(DispenseError::OneOrNoCansLeft);
            }
        }
        self.force_dispense(addr).await
    }

    pub async fn force_dispense(&mut self, addr: DispenserAddress) -> DispenseResult {
        debug!("Driving dispense motor at {}{}", addr.row, addr.col);
        self.drive_motor(addr).await;
        let home_gpio_index = MotorDriver::motor_homed_gpio_index(addr);
        debug!("Motor homed gpio index is is {}", home_gpio_index);
        debug!("Waiting for motor to leave home");

        self.output_enable.set_low();
        //Buffer seems to need time to 'settle'
        Timer::after_micros(20).await;

        let b = self.bus[home_gpio_index]
            .wait_for_low()
            .with_timeout(Duration::from_millis(1000))
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

        //Avoid issue with bouncing microswitch contacts
        Timer::after_millis(500).await;

        //Now the motor is moving, it has 3 seconds to return home to complete the vend cycles
        let b = self.bus[home_gpio_index]
            .wait_for_high()
            .with_timeout(Duration::from_millis(2500))
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
            Err(DispenseError::MotorStuckNotHome)
        }
    }

    pub async fn set_chiller_on(&mut self, status: bool) {
        self.chiller_on = status;
        //This will cause the status to be actioned.
        self.stop_motors().await;
    }
}
