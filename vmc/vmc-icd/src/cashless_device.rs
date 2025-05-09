use postcard_schema::Schema;
use serde::{Deserialize, Serialize};

use crate::DispenserAddress;

//These are used as part of the ICD for features relating to the MDB Cashless Device
//They are basically simplified copies of the MDB structs in the mdb-async::cashless_device 

#[derive(Serialize, Deserialize, Schema, Debug, PartialEq, Copy, Clone)]
pub enum CashlessDeviceCommand {
    Reset,
    Enable,
    Disable,
    RecordCashTransaction(u16, DispenserAddress),
    StartTransaction(u16, DispenserAddress),
    CancelTransaction,
    VendSuccess(DispenserAddress),
    VendFailed,
}

#[derive(Serialize, Deserialize, Schema, Debug, PartialEq, Copy, Clone)]
pub enum CashlessDeviceEvent {
    Available,
    Unavailable,
    VendApproved(u16),
    VendDenied,
}

pub type CashlessResult = Result<(), ()>;

