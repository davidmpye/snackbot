//These structs are used to communicate with the dispenser aka motor driver
//functionality - to find out details about the machine, and to drive the 
//dispense functionality
use postcard_schema::Schema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Schema, Debug, PartialEq, Copy, Clone)]
pub struct DispenserAddress {
    pub row: char,
    pub col: char,
}

//Information about a dispenser at a particular address
//Will return None if the motor is not present
pub type DispenserOption = Option<Dispenser>;
#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub struct Dispenser {
    pub address: DispenserAddress,
    pub dispenser_type: DispenserType,
    pub motor_status: MotorStatus,
    pub can_status: Option<CanStatus>,
}

#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub enum DispenserType {
    Spiral,
    Can,
}

#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub enum MotorStatus {
    Ok,
    MotorNotHome,
    //If motor not present, Dispenser Option would just be None
}

#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub enum CanStatus {
    Ok,
    LastCan,
}

//The result of attempting a vend operation
pub type DispenseResult = Result<(), DispenseError>;

#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub enum DispenseError {
    MotorNotPresent,
    MotorNotHome,
    MotorStuckHome,
    MotorStuckNotHome,
    OneOrNoCansLeft, //Can vendor won't vend if only one can present
    NoDropDetected,  //not implemented yet - my machine does not support
    InvalidAddress,
}
