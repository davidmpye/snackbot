use postcard_schema::Schema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Schema, Debug, PartialEq, Copy, Clone)]
pub struct ChillerStatus {
    on: bool,
    current_temperature: f32,
    setpoint: f32,
}

