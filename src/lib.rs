use image::RgbaImage;
use serde::{Deserialize, Serialize};

pub mod vision_model;

/// The width of a stone in pixels on the normalized image of the board.
pub const STONE_SIZE: u32 = 16;

/// The messages that can be sent to the control websocket.
#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ControlMessage {
    NewTrainingPattern,
}

/// Deserializes an image from a binary message.
/// The first 4 bytes of the message are the width of the image.
/// The next 4 bytes are the height of the image.
/// The rest of the message is the image data, in 32-bit RGBA format.
pub fn deserialize_image(message: Vec<u8>) -> RgbaImage {
    let width = u32::from_be_bytes(message[0..4].try_into().unwrap());
    let height = u32::from_be_bytes(message[4..8].try_into().unwrap());
    let data = message[8..].to_vec();
    RgbaImage::from_raw(width, height, data).unwrap()
}
