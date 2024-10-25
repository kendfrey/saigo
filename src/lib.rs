use serde::{Deserialize, Serialize};

/// The width of a stone in pixels on the normalized image of the board.
pub const STONE_SIZE: u32 = 16;

/// The messages that can be sent to the control websocket.
#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ControlMessage {
    NewTrainingPattern,
}
