#![cfg_attr(not(feature = "use-std"), no_std)]
use postcard_rpc::{endpoints, topics, TopicDirection};
use postcard_schema::Schema;
use serde::{Deserialize, Serialize};

pub mod dispenser;
use crate::dispenser::*;

pub mod coinacceptor;
use crate::coinacceptor::*;


#[derive(Serialize, Deserialize, Schema, Debug, PartialEq)]
pub struct ChillerStatus {
    temp_setpoint: f32,    
    current_temp: f32,
    chiller_on: bool,
    chiller_duty_cycle: u8,  //Averaged over preceding 24 hours as a %
}

endpoints! {
    list = ENDPOINT_LIST;
    omit_std = true;
    | EndpointTy              | RequestTy        | ResponseTy           | Path              |
    | ----------              | ---------        | ----------           | ----              |
    | GetDispenserInfo        | DispenserAddress | DispenserOption      | "dispenserinfo"   |   //Get current state of dispenser at a row/col address
    | Dispense                | DispenserAddress | DispenseResult       | "dispence"        |
    | ForceDispense           | DispenserAddress | DispenseResult       | "forcedispense"   |   //Attempt vend regardless of initial state



    //Coin acceptor enable/disable
    | SetCoinAcceptorEnabled  | bool             | ()                   | "setcoinacceptorenabled" |

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
    | TopicTy                   | MessageTy             | Path                             | Cfg                           |
    | -------                   | ---------             | ----                             | ---                           |
    | CoinInsertedTopic         | CoinInserted          | "/mdb/coinacceptor/coininserted" |                               |
    | EventTopic                | CoinAcceptorEvent     | "/mdb/coinacceptor/event"        |                               |
}
