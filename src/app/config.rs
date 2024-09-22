use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use image::RgbImage;
use serde::{Deserialize, Serialize};

/// All persisted settings for the application.
#[derive(Default, Serialize, Deserialize)]
pub struct Config {
    pub board: BoardConfig,
    pub display: DisplayConfig,
    pub camera: CameraConfig,
}

impl Config {
    const REFERENCE_IMAGE_FILE: &'static str = "reference.png";

    /// Saves a lightweight subset of the configuration to the current profile.
    pub fn save_fast(&self) -> Result<(), String> {
        self.save(None, true)
    }

    /// Saves the configuration to the file system.
    pub fn save(&self, profile: Option<&str>, fast: bool) -> Result<(), String> {
        let full_path = get_configuration_file_path(profile)?;
        confy::store_path(full_path, self).map_err(|e| e.to_string())?;
        if !fast {
            self.save_reference_image(profile)?;
        }
        Ok(())
    }

    /// Saves the reference image to the file system.
    pub fn save_reference_image(&self, profile: Option<&str>) -> Result<(), String> {
        let full_path =
            get_configuration_file_path(profile)?.with_file_name(Self::REFERENCE_IMAGE_FILE);
        match &self.camera.reference_image {
            Some(reference_image) => reference_image.save(full_path).map_err(|e| e.to_string()),
            None => fs::remove_file(full_path).map_err(|e| e.to_string()),
        }
    }

    /// Loads the configuration from the file system.
    pub fn load(profile: Option<&str>) -> Result<Self, String> {
        let full_path = get_configuration_file_path(profile)?;
        let mut config = confy::load_path::<Self>(full_path.clone()).map_err(|e| e.to_string())?;
        match image::open(full_path.with_file_name(Self::REFERENCE_IMAGE_FILE)) {
            Ok(reference_image) => {
                config.camera.reference_image = Some(reference_image.into_rgb8())
            }
            Err(_) => config.camera.reference_image = None,
        }
        Ok(config)
    }
}

/// Gets the list of available configuration profiles.
pub fn get_profiles() -> Result<Vec<String>, String> {
    let entries = get_configuration_file_path(None)?
        .parent()
        .unwrap()
        .read_dir()
        .map_err(|e| e.to_string())?;
    let dirs = entries.filter_map(|entry| {
        entry.ok().and_then(|entry| {
            if entry.file_type().ok()?.is_dir() {
                Some(entry.file_name().into_string().ok()?)
            } else {
                None
            }
        })
    });
    Ok(dirs.collect())
}

/// Gets the full path to the configuration file for the given profile name.
fn get_configuration_file_path(profile: Option<&str>) -> Result<PathBuf, String> {
    let mut full_path =
        confy::get_configuration_file_path("Saigo", "config").map_err(|e| e.to_string())?;

    // If a profile is provided, add it to the directory without changing the file name
    if let Some(profile) = profile {
        if !is_single_normal_component(profile) {
            return Err("The path must be a valid folder name.".to_string());
        }

        let file_name = full_path.file_name().unwrap().to_os_string();
        full_path.pop();
        full_path.push(profile);
        full_path.push(file_name);
    }

    Ok(full_path)
}

/// Checks that the path contains a single component, and that it is a normal component.
fn is_single_normal_component<P: AsRef<Path>>(path: P) -> bool {
    let mut components = path.as_ref().components();
    match components.next() {
        Some(Component::Normal(_)) => components.next().is_none(),
        _ => false,
    }
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
