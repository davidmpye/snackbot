use postcard_schema::Schema;
use serde::{Deserialize, Serialize};

//These are used as part of the ICD for features relating to the MDB Coin Acceptor.
//They are basically simplified copies of the MDB structs in the mdb-async::coin_acceptor

#[derive(Serialize, Deserialize, Schema, Debug, PartialEq, Copy, Clone)]
pub struct CoinInserted {
    pub value: u16, //Coin value
    pub routing: CoinRouting, //Where it went
}

#[derive(Serialize, Deserialize, Schema, Debug, PartialEq, Copy, Clone)]
pub enum CoinRouting {
    CashBox,
    Tube,
    Reject,
    Unknown,
}


#[derive(Serialize, Deserialize, Schema, Debug, PartialEq, Copy, Clone)]
pub enum CoinAcceptorEvent {
    EscrowPressed,
}
