use postcard_schema::Schema;
use serde::{Deserialize, Serialize};

use crate::DispenserAddress;

//These are used as part of the ICD for features relating to the MDB Cashless Device
//They are basically simplified copies of the MDB structs in the mdb-async::cashless_device 

#[derive(Serialize, Deserialize, Schema, Debug, PartialEq, Copy, Clone)]
pub enum CashlessDeviceCommand {
    StartTransaction(u16, DispenserAddress),
    CancelTransaction,
    EnableDevice,
    DisableDevice,
    EndSession,
    VendSuccess(DispenserAddress),
    VendFailed,
    RecordCashTransaction(u16, DispenserAddress),
}

pub type CashlessResult = Result<(), ()>;

