use goban::pieces::{stones::Color, util::coord::Coord};
use image::RgbaImage;
use serde::{
    de::{self, Unexpected},
    Deserialize, Deserializer, Serialize, Serializer,
};

pub mod vision_model;

/// The width of a stone in pixels on the normalized image of the board.
pub const STONE_SIZE: u32 = 16;

/// A location on the board in SGF notation.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct SgfCoord(pub String);

const SGF_CHAR_MAP: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";

impl TryFrom<&SgfCoord> for Coord {
    type Error = ();
    fn try_from(value: &SgfCoord) -> Result<Self, Self::Error> {
        let x = SGF_CHAR_MAP
            .find(value.0.chars().nth(0).ok_or(())?)
            .ok_or(())?;
        let y = SGF_CHAR_MAP
            .find(value.0.chars().nth(1).ok_or(())?)
            .ok_or(())?;
        Ok((x as u8, y as u8))
    }
}

impl TryFrom<Coord> for SgfCoord {
    type Error = ();
    fn try_from(value: Coord) -> Result<Self, Self::Error> {
        let x = SGF_CHAR_MAP.chars().nth(value.0 as usize).ok_or(())?;
        let y = SGF_CHAR_MAP.chars().nth(value.1 as usize).ok_or(())?;
        Ok(SgfCoord(format!("{}{}", x, y)))
    }
}

/// The player color.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct SerializableColor(pub Color);

impl Serialize for SerializableColor {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self.0 {
            Color::Black => serializer.serialize_str("B"),
            Color::White => serializer.serialize_str("W"),
        }
    }
}

impl<'de> Deserialize<'de> for SerializableColor {
    fn deserialize<D>(deserializer: D) -> Result<SerializableColor, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.to_uppercase().chars().nth(0) {
            Some('B') => Ok(SerializableColor(Color::Black)),
            Some('W') => Ok(SerializableColor(Color::White)),
            _ => Err(de::Error::invalid_value(Unexpected::Str(&s), &"B or W")),
        }
    }
}

/// The messages that can be sent to the control websocket.
#[derive(Clone, PartialEq, Eq, Hash, Debug, Default, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ControlMessage {
    #[default]
    Reset,
    NewTrainingPattern,
    NewGame {
        user_color: SerializableColor,
    },
    PlayMove {
        location: SgfCoord,
    },
    PlayPass,
    EndGame {
        winner: SerializableColor,
    },
}

/// The messages that can be returned from the game websocket.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GameMessage {
    Move { location: SgfCoord },
    Pass,
    Resign,
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
