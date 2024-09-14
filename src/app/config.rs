use image::RgbImage;
use serde::{Deserialize, Serialize};

/// All persisted settings for the application.
#[derive(Default, Serialize, Deserialize)]
pub struct Config {
    pub board: BoardConfig,
    pub display: DisplayConfig,
    pub camera: CameraConfig,
}

/// The settings for the game board itself.
#[derive(Clone, Serialize, Deserialize)]
pub struct BoardConfig {
    pub width: u32,
    pub height: u32,
}

impl Default for BoardConfig {
    fn default() -> Self {
        Self {
            width: 19,
            height: 19,
        }
    }
}

/// The settings used to render the display.
#[derive(Clone, Serialize, Deserialize)]
pub struct DisplayConfig {
    pub image_width: u32,
    pub image_height: u32,
    pub angle: f32,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub perspective_x: f32,
    pub perspective_y: f32,
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            image_width: 640,
            image_height: 360,
            angle: 0.0,
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
            perspective_x: 0.0,
            perspective_y: 0.0,
        }
    }
}

/// The settings used to read the board from the camera.
#[derive(Clone, Serialize, Deserialize)]
pub struct CameraConfig {
    pub device: String,
    pub width: u32,
    pub height: u32,
    pub top_left: Point,
    pub top_right: Point,
    pub bottom_left: Point,
    pub bottom_right: Point,
    #[serde(skip)]
    pub reference_image: Option<RgbImage>,
}

impl Default for CameraConfig {
    fn default() -> Self {
        Self {
            device: String::new(),
            width: 640,
            height: 360,
            top_left: Point { x: 0.36, y: 0.25 },
            top_right: Point { x: 0.64, y: 0.25 },
            bottom_left: Point { x: 0.36, y: 0.75 },
            bottom_right: Point { x: 0.64, y: 0.75 },
            reference_image: None,
        }
    }
}

/// A point in 2D space.
#[derive(Clone, Serialize, Deserialize)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}
