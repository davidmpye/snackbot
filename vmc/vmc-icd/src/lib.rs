#![cfg_attr(not(feature = "use-std"), no_std)]
use postcard_rpc::{endpoints, topics, TopicDirection};
use postcard_schema::Schema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub struct ChillerStatus {
    temp_setpoint: f32,    
    current_temp: f32,
    chiller_on: bool,
    chiller_duty_cycle: u8,  //Averaged over preceding 24 hours as a %
}

#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
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
    Unknown,
    //If motor not present, Dispenser Option would just be None
}

#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub enum CanStatus {
    Ok,
    LastCan,
    Unknown,
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

endpoints! {
    list = ENDPOINT_LIST;
    omit_std = true;
    | EndpointTy              | RequestTy        | ResponseTy           | Path              |
    | ----------              | ---------        | ----------           | ----              |
    | GetChillerStatus        | ()               | ChillerStatus        | "chillerstatus"   |
    | SetChillerTemp          | f32              | bool                 | "setchillertemp"  |
    | GetDispenserInfo        | DispenserAddress | DispenserOption      | "dispenserinfo"   |   //Get current state of dispenser at a row/col address
    | Dispense                | DispenserAddress | DispenseResult       | "dispence"        |
    | ForceDispense           | DispenserAddress | DispenseResult       | "forcedispense"   |   //Attempt vend regardless of initial state
}

topics! {
    list = TOPICS_IN_LIST;
    direction = TopicDirection::ToServer;
    | TopicTy                   | MessageTy     | Path              |
    | -------                   | ---------     | ----              |
}

topics! {
    list = TOPICS_OUT_LIST;
    direction = TopicDirection::ToClient;
    | TopicTy                   | MessageTy     | Path              | Cfg                           |
    | -------                   | ---------     | ----              | ---                           |
}
