use postcard_schema::Schema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Schema, Debug, PartialEq, Copy, Clone)]
pub struct ChillerInfo {
    pub target_temp: u8,
    pub current_temp: u8,
    pub duty_cycle: u8,
    pub compressor_status: bool,
}

