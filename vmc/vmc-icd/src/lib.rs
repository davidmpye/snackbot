#![cfg_attr(not(feature = "use-std"), no_std)]
use postcard_rpc::{endpoints, topics, TopicDirection};

pub mod dispenser;
use crate::dispenser::*;

pub mod coinacceptor;
use crate::coinacceptor::*;

pub mod chiller;
use crate::chiller::*;

endpoints! {
    list = ENDPOINT_LIST;
    omit_std = true;
    | EndpointTy              | RequestTy        | ResponseTy           | Path             |
    | ----------              | ---------        | ----------           | ----             |
    //Things to operate the motor driver
    | DispenseEndpoint        | DispenseCommand  | DispenseResult       | "/dispenser/dispense"    |  //Dispenses or force-dispenses an item
    | DispenserStatusEndpoint | DispenserAddress | DispenserOption      | "/dispenser/status"      |  //Get the status for a given dispenser

    //Control the chiller
 //   | SetChillerTemp          | u8               | bool                 | "/chiller/settemp" |  //Set the target temperature for the chiller (fixed point - eg 255 = 25.5'C)
 //   | GetChillerInfo          | ()               | ChillerInfo          | "/chiller/status"    |  //Get the chiller info

 //   | CoinAcceptorInfoEndpoint   | ()            | CoinAcceptor         | "/mdb/coinacceptor/info"   | //Returns coinacceptor
    | CoinAcceptorEnableEndpoint | bool          | ()                   | "/mdb/coinacceptor/enable" | //Whether acceptor should accept coins

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
