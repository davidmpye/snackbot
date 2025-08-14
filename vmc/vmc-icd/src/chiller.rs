use postcard_schema::Schema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Schema, Debug, PartialEq, Copy, Clone)]
pub struct ChillerStatus {
    pub on: bool,
    pub current_temperature: f32,
    pub setpoint: f32,
}