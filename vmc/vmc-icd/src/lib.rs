#![cfg_attr(not(feature = "use-std"), no_std)]
use postcard_rpc::{endpoints, topics, TopicDirection};
use postcard_schema::Schema;
use serde::{Deserialize, Serialize};

pub mod dispenser;
use crate::dispenser::*;

pub mod coinacceptor;
use crate::coinacceptor::*;

endpoints! {
    list = ENDPOINT_LIST;
    omit_std = true;
    | EndpointTy              | RequestTy        | ResponseTy           | Path              |
    | ----------              | ---------        | ----------           | ----              |
    | DispenseEndpoint        | DispenserCommand | ()                   | "dispenser"       |

    //Coin acceptor endpoints
    | SetCoinAcceptorEnabled  | bool             | ()                   | "setcoinacceptorenabled"  |
    | DispenseCoins           | u16              | u16                  | "dispensecoins"           | //Returns amount ACTUALLY refunded

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
