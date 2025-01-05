#![cfg_attr(not(feature = "use-std"), no_std)]

use postcard_rpc::{endpoints, topics, TopicDirection};
use postcard_schema::Schema;
use serde::{Deserialize, Serialize};

endpoints! {
    list = ENDPOINT_LIST;
    omit_std = true;
    | EndpointTy              | RequestTy        | ResponseTy           | Path              |
    | ----------              | ---------        | ----------           | ----              |
    | SetBacklight            | bool             | ()                   | "setBacklight"    |
    | SetLine1Text            | [u8;32]          | ()                   | "setLine1Text"    |
    | SetLine2Text            | [u8;32]          | ()                   | "setLine2Text"    |
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
    | ServiceModeTopic          | bool          | "serviceMode"     |                               |
}

