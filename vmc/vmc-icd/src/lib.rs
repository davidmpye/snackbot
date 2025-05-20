#![cfg_attr(not(feature = "use-std"), no_std)]
use postcard_rpc::{endpoints, topics, TopicDirection};

pub mod dispenser;
use crate::dispenser::*;

pub mod coin_acceptor;
use crate::coin_acceptor::*;

pub mod cashless_device;
use crate::cashless_device::*;

pub mod chiller;
use crate::chiller::*;
use serde::{Deserialize, Serialize};
use postcard_schema::Schema;


#[derive(Serialize, Deserialize, Schema, Debug, PartialEq,Copy, Clone)]
pub struct VendCommand {
    row: u8,
    col: u8,
    price: u16,  //Unscaled, GB pence 
}

//These are the reasons a vend might fail
#[derive(Serialize, Deserialize, Schema, Debug, PartialEq,Copy, Clone)]
pub enum VendError {
    MotorNotPresent,
    MotorNotHome,
    MotorStuckHome,
    MotorStuckNotHome,
    OneOrNoCansLeft, //Can vendor in my model won't (willingly) vend if only one can present
    NoDropDetected, 
    InvalidAddress,
    Cancelled, 
}
pub type VendResult = Result<(), VendError>;


#[derive(Serialize, Deserialize, Schema, Debug, PartialEq,Copy, Clone)]
pub struct ChillerStatus {
    on: bool,
    current_temperature: f32,
    setpoint: f32,
}

endpoints! {
    list = ENDPOINT_LIST;
    omit_std = true;
    | EndpointTy              | RequestTy        | ResponseTy           | Path                 |
    | ----------              | ---------        | ----------           | ----                 |
    | ItemAvailable           | VendCommand      | VendResult           | "/vmc/itemavailable" | //Test if the item is available to vend?  
    | Vend                    | VendCommand      | VendResult           | "/vmc/vend"          | //Vend the item
    | ForceDispense           | VendCommand      | VendResult           | "/vmc/forcedispense" | //NB THIS DOES NOT CHARGE THE USER
    | CancelVend              | ()               | VendResult           | "/vmc/cancelvend"    | //Cancel a vend that is in progress

    //There will be other ones so you can find out about the peripherals etc

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
    | TopicTy              | MessageTy             | Path                             | Cfg                           |
    | -------              | ---------             | ----                             | ---                           |  
    | Chiller              | ChillerStatus         | "/vmc/status/chiller"            |                               | 
}
