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

struct VendCommand {
    row: u8,
    col: u8,
    price: u16,
}

endpoints! {
    list = ENDPOINT_LIST;
    omit_std = true;
    | EndpointTy              | RequestTy        | ResponseTy           | Path             |
    | ----------              | ---------        | ----------           | ----             |
    //Things to operate the motor driver
    | DispenseEndpoint        | DispenseCommand  | DispenseResult       | "/dispenser/dispense"    |  //Dispenses or force-dispenses an item
    | DispenserStatusEndpoint | DispenserAddress | DispenserOption      | "/dispenser/status"      |  //Get the status for a given dispenser

    | CoinAcceptorEnableEndpoint | bool          | ()                   | "/mdb/coinacceptor/enable" | //Whether acceptor should accept coins

    | CashlessDeviceCmdEndpoint  | CashlessDeviceCommand | ()    | "/mdb/cashlessdevice/cmd"  | //Commands to the cashless device
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
    //An event from the cashless device
    | CashlessEventTopic        | CashlessDeviceEvent   | "/mdb/cashless/event"            |                               |
}
