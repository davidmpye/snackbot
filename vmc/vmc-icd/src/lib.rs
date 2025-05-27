#![cfg_attr(not(feature = "use-std"), no_std)]
use postcard_rpc::{endpoints, topics, TopicDirection};



pub mod coin_acceptor;
use crate::coin_acceptor::*;


pub mod chiller;
use crate::chiller::*;
use serde::{Deserialize, Serialize};
use postcard_schema::Schema;


#[derive(Serialize, Deserialize, Schema, Debug, PartialEq,Copy, Clone)]
pub struct VendCommand {
    pub row: u8,
    pub col: u8,
    pub price: u16,  //Unscaled, GB pence 
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
    PaymentFailed,
    CommsFault
}

pub type VendResult = Result<(), VendError>;


//These are sent as topics during the Vend Process to give the UI a chance to update
//So, when you call Vend, as the VMC progresses through taking payment
#[derive(Serialize, Deserialize, Schema, Debug, PartialEq,Copy, Clone)]
pub enum VendProgress {
    AwaitingPayment,
    Dispensing,
    //No need for a Complete, because when the Vend endpoint completes, it will return a VendResult to you
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
    | ChillerTopic         | ChillerStatus         | "/vmc/status/chiller"            |                               | 
    | VendProgressTopic    | VendProgress          | "/vmc/vend_progress"             |                               |
}
