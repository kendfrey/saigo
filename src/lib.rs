use std::ops::Not;

use goban::pieces::{stones::Color, util::coord::Coord};
use image::RgbaImage;
use serde::{Deserialize, Serialize};

pub mod vision_model;

/// The width of a stone in pixels on the normalized image of the board.
pub const STONE_SIZE: u32 = 16;

/// A location on the board in SGF notation.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct SgfCoord(pub String);

const SGF_CHAR_MAP: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";

impl TryFrom<&SgfCoord> for Coord {
    type Error = String;
    fn try_from(value: &SgfCoord) -> Result<Self, Self::Error> {
        let x = SGF_CHAR_MAP
            .find(value.0.chars().nth(0).ok_or("Invalid coordinate")?)
            .ok_or("Invalid coordinate")?;
        let y = SGF_CHAR_MAP
            .find(value.0.chars().nth(1).ok_or("Invalid coordinate")?)
            .ok_or("Invalid coordinate")?;
        Ok((x as u8, y as u8))
    }
}

impl TryFrom<Coord> for SgfCoord {
    type Error = String;
    fn try_from(value: Coord) -> Result<Self, Self::Error> {
        let x = SGF_CHAR_MAP
            .chars()
            .nth(value.0 as usize)
            .ok_or("Invalid coordinate")?;
        let y = SGF_CHAR_MAP
            .chars()
            .nth(value.1 as usize)
            .ok_or("Invalid coordinate")?;
        Ok(SgfCoord(format!("{}{}", x, y)))
    }
}

const GTP_CHAR_MAP: &str = "ABCDEFGHJKLMNOPQRSTUVWXYZ";

impl SgfCoord {
    /// Creates an SGF-style coordinate from a GTP-style coordinate.
    pub fn from_gtp_coord(coord: &str, board_height: u8) -> Result<Self, String> {
        let x = GTP_CHAR_MAP
            .find(
                coord
                    .chars()
                    .nth(0)
                    .ok_or("Invalid coordinate")?
                    .to_ascii_uppercase(),
            )
            .ok_or("Invalid coordinate")? as u8;
        let y = board_height - coord[1..].parse::<u8>().map_err(|e| format!("{}", e))?;
        SgfCoord::try_from((x, y))
    }

    /// Converts the SGF-style coordinate to a GTP-style coordinate.
    pub fn to_gtp_coord(&self, board_height: u8) -> Result<String, String> {
        let (x, y) = Coord::try_from(self)?;
        Ok(format!(
            "{}{}",
            GTP_CHAR_MAP
                .chars()
                .nth(x as usize)
                .ok_or("Invalid coordinate")?,
            board_height - y
        ))
    }
}

/// The player color.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
#[serde(try_from = "&str")]
pub enum SerializableColor {
    #[serde(rename = "B")]
    Black,
    #[serde(rename = "W")]
    White,
}

impl Not for SerializableColor {
    type Output = Self;
    fn not(self) -> Self::Output {
        match self {
            SerializableColor::Black => SerializableColor::White,
            SerializableColor::White => SerializableColor::Black,
        }
    }
}

impl From<Color> for SerializableColor {
    fn from(color: Color) -> Self {
        match color {
            Color::Black => SerializableColor::Black,
            Color::White => SerializableColor::White,
        }
    }
}

impl From<SerializableColor> for Color {
    fn from(color: SerializableColor) -> Self {
        match color {
            SerializableColor::Black => Color::Black,
            SerializableColor::White => Color::White,
        }
    }
}

impl TryFrom<&str> for SerializableColor {
    type Error = &'static str;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.to_uppercase().chars().nth(0) {
            Some('B') => Ok(SerializableColor::Black),
            Some('W') => Ok(SerializableColor::White),
            _ => Err("Expected B or W"),
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
        #[serde(rename = "move")]
        move_: PlayerMove,
    },
}

/// A move and the player who made it.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PlayerMove {
    #[serde(flatten)]
    pub move_: Move,
    pub player: SerializableColor,
}

/// A move on the board, pass, or resignation.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Move {
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
