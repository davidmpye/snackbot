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
    PayoutBusy,
    NoCredit,
    DefectiveTubeSensor,
    DoubleArrival,
    AcceptorUnplugged,
    TubeJam,
    RomChecksumError,
    CoinRoutingError,
    Busy,
    WasReset,
    CoinJam,
    AttemptedCoinRemoval,
    InvalidEvent,
}


impl From<u8> for CoinAcceptorEvent {
    fn from(byte:u8) -> Self {
        match byte {
            0x01 => CoinAcceptorEvent::EscrowPressed,
            0x02 => CoinAcceptorEvent::PayoutBusy,
            0x03 => CoinAcceptorEvent::NoCredit,
            0x04 => CoinAcceptorEvent::DefectiveTubeSensor,
            0x05 => CoinAcceptorEvent::DoubleArrival,
            0x06 => CoinAcceptorEvent::AcceptorUnplugged,
            0x07 => CoinAcceptorEvent::TubeJam,
            0x08 => CoinAcceptorEvent::RomChecksumError,
            0x09 => CoinAcceptorEvent::CoinRoutingError,
            0x0A => CoinAcceptorEvent::Busy,
            0x0B => CoinAcceptorEvent::WasReset,
            0x0C => CoinAcceptorEvent::CoinJam,
            0x0D => CoinAcceptorEvent::InvalidEvent,
            _ => CoinAcceptorEvent::InvalidEvent,
        }
    }
}