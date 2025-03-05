#![cfg_attr(not(feature = "use-std"), no_std)]
use postcard_rpc::{endpoints, topics, TopicDirection};

pub mod dispenser;
use crate::dispenser::*;

pub mod coinacceptor;
use crate::coinacceptor::*;

endpoints! {
    list = ENDPOINT_LIST;
    omit_std = true;
    | EndpointTy              | RequestTy        | ResponseTy           | Path              |
    | ----------              | ---------        | ----------           | ----              |
    | DispenseEndpoint        | DispenseCommand | DispenseResult       | "dispenser"       |


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
