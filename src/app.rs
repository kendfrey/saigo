use std::{
    sync::{Arc, RwLock},
    time::Duration,
};

use config::{CameraConfig, Config, DisplayConfig};
use image::{Rgb, RgbImage, Rgba, RgbaImage};
use imageproc::{
    drawing::draw_filled_circle_mut,
    geometric_transformations::{warp, warp_into, Interpolation, Projection},
};
use nokhwa::{
    pixel_format::RgbFormat,
    utils::{ApiBackend, CameraFormat, RequestedFormat, RequestedFormatType, Resolution},
    Camera,
};
use tokio::{
    sync::watch,
    time::{self, MissedTickBehavior},
};

pub mod config;

/// The width of a stone in pixels on the normalized image of the board.
const STONE_SIZE: u32 = 16;

/// The global state of the application.
pub struct AppState {
    config: Config,
    display_dirty: watch::Sender<()>,
    display_broadcast: watch::Sender<RgbaImage>,
    camera_dirty: watch::Sender<()>,
    camera_broadcast: watch::Sender<RgbImage>,
}

impl AppState {
    /// Starts a new instance of the application.
    pub fn start() -> Arc<RwLock<Self>> {
        let (display_dirty, _) = watch::channel(());
        let (display_broadcast, _) = watch::channel(RgbaImage::new(160, 120));
        let (camera_dirty, _) = watch::channel(());
        let (camera_broadcast, _) = watch::channel(RgbImage::new(160, 120));
        let state = Self {
            config: Config::default(),
            display_dirty,
            display_broadcast,
            camera_dirty,
            camera_broadcast,
        };
        let state_ref = Arc::new(RwLock::new(state));
        Self::spawn_render_loop(state_ref.clone());
        Self::spawn_camera_loop(state_ref.clone());
        state_ref
    }

    /// Returns a new receiver for the display broadcast channel.
    pub fn subscribe_to_display_broadcast(&self) -> watch::Receiver<RgbaImage> {
        self.display_broadcast.subscribe()
    }

    /// Returns a new receiver for the camera broadcast channel.
    pub fn subscribe_to_camera_broadcast(&self) -> watch::Receiver<RgbImage> {
        self.camera_broadcast.subscribe()
    }

    /// Gets the current display configuration.
    pub fn get_display_config(&self) -> &DisplayConfig {
        &self.config.display
    }

    /// Sets the display configuration.
    pub fn set_display_config(&mut self, display: DisplayConfig) {
        if display.image_width == 0 || display.image_height == 0 {
            return;
        }

        self.config.display = display;
        self.display_dirty.send_replace(());
    }

    /// Gets the current camera configuration.
    pub fn get_camera_config(&self) -> &CameraConfig {
        &self.config.camera
    }

    /// Sets the camera configuration.
    pub fn set_camera_config(&mut self, camera: CameraConfig) {
        // Copy the old reference image because it's not included in the serialized config
        let old_reference = self.config.camera.reference_image.clone();

        // If the camera settings change, reset the camera
        let should_reset = self.config.camera.device != camera.device
            || self.config.camera.width != camera.width
            || self.config.camera.height != camera.height;

        self.config.camera = camera;
        self.config.camera.reference_image = old_reference;

        if should_reset {
            self.camera_dirty.send_replace(());
        }
    }

    /// Captures a reference image of the board.
    pub fn take_reference_image(&mut self) {
        self.config.camera.reference_image =
            Some(self.to_board_image(&self.camera_broadcast.borrow()));
    }

    /// Transforms the camera image to the normalized board image.
    fn to_board_image(&self, frame: &RgbImage) -> RgbImage {
        // Buffer to copy the board image to
        let mut board_image = RgbImage::new(
            self.config.board.width * STONE_SIZE,
            self.config.board.height * STONE_SIZE,
        );

        // Transform the image coordinates to between 0 and 1, since that's how the control points are respresented
        let normalize_transform =
            Projection::scale(1.0 / frame.width() as f32, 1.0 / frame.height() as f32);

        // Transform from the 0-1 coordinate system to the final board image
        let perspective_transform = Projection::from_control_points(
            [
                (self.config.camera.top_left.x, self.config.camera.top_left.y),
                (
                    self.config.camera.top_right.x,
                    self.config.camera.top_right.y,
                ),
                (
                    self.config.camera.bottom_left.x,
                    self.config.camera.bottom_left.y,
                ),
                (
                    self.config.camera.bottom_right.x,
                    self.config.camera.bottom_right.y,
                ),
            ],
            [
                (STONE_SIZE as f32 * 0.5, STONE_SIZE as f32 * 0.5),
                (
                    STONE_SIZE as f32 * (self.config.board.width as f32 - 0.5),
                    STONE_SIZE as f32 * 0.5,
                ),
                (
                    STONE_SIZE as f32 * 0.5,
                    STONE_SIZE as f32 * (self.config.board.height as f32 - 0.5),
                ),
                (
                    STONE_SIZE as f32 * (self.config.board.width as f32 - 0.5),
                    STONE_SIZE as f32 * (self.config.board.height as f32 - 0.5),
                ),
            ],
        )
        .unwrap_or(normalize_transform.invert());

        // Warp the camera frame to the board image
        warp_into(
            &frame,
            &normalize_transform.and_then(perspective_transform),
            Interpolation::Bilinear,
            Rgb([0, 0, 0]),
            &mut board_image,
        );

        board_image
    }

    /// Spawns the renderer in a background task.
    fn spawn_render_loop(state_ref: Arc<RwLock<AppState>>) {
        tokio::spawn(async move {
            let broadcast;
            let mut dirty_receiver;
            {
                let state = state_ref.read().unwrap();
                broadcast = state.display_broadcast.clone();
                // Subscribe to state updates
                dirty_receiver = state.display_dirty.subscribe();
                dirty_receiver.mark_changed();
            }

            // Wait for the state to update
            loop {
                match dirty_receiver.changed().await {
                    Ok(()) => {
                        // If the state changes, rerender the display
                        broadcast.send_replace(render(&state_ref.read().unwrap().config));
                    }
                    Err(_) => {
                        // If the channel is closed, stop rendering
                        return;
                    }
                }
            }
        });
    }

    /// Spawns the camera capture in a background task.
    fn spawn_camera_loop(state_ref: Arc<RwLock<AppState>>) {
        tokio::spawn(async move {
            let camera_broadcast;
            let mut dirty_receiver;
            {
                let state = state_ref.read().unwrap();
                camera_broadcast = state.camera_broadcast.clone();
                // Subscribe to camera configuration changes that require a reset
                dirty_receiver = state.camera_dirty.subscribe();
                dirty_receiver.mark_changed();
            }

            let mut camera: Option<Camera> = None;

            // Limit the frame rate to 10 FPS
            let mut interval = time::interval(Duration::from_millis(100));
            interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
            loop {
                interval.tick().await;

                // Save work if no one is listening anyway
                if camera_broadcast.is_closed() {
                    continue;
                }

                // If the camera capture settings change, reset the camera
                if dirty_receiver.has_changed().unwrap() {
                    dirty_receiver.mark_unchanged();
                    camera = start_camera(&state_ref.read().unwrap().config.camera);
                }

                // Try to capture a frame
                if let Some(frame) = read_frame(camera.as_mut()) {
                    // Broadcast the raw frame
                    camera_broadcast.send_replace(frame);
                }
            }
        });
    }
}

/// Renders the display.
fn render(config: &Config) -> RgbaImage {
    let raw = render_raw(config);
    let proj = get_display_projection(config);
    warp(&raw, &proj, Interpolation::Bilinear, Rgba([0, 0, 0, 0]))
}

/// Renders the display in a normalized position.
/// This will later be warped according to the display configuration.
fn render_raw(config: &Config) -> RgbaImage {
    let stone_size = stone_size(config);
    let origin_x =
        (config.display.image_width as f32 - stone_size * config.board.width as f32) * 0.5;
    let origin_y =
        (config.display.image_height as f32 - stone_size * config.board.height as f32) * 0.5;

    // Draw a dot on every intersection
    let mut img = RgbaImage::new(config.display.image_width, config.display.image_height);
    for x in 0..config.board.width {
        for y in 0..config.board.height {
            let ctr_x = origin_x + (x as f32 + 0.5) * stone_size;
            let ctr_y = origin_y + (y as f32 + 0.5) * stone_size;
            draw_filled_circle_mut(
                &mut img,
                (ctr_x as i32, ctr_y as i32),
                (stone_size * 0.125) as i32,
                Rgba([255, 255, 255, 255]),
            );
        }
    }

    // Draw a green circle in the top left corner for orientation
    let ctr_x = origin_x + stone_size * 0.5;
    let ctr_y = origin_y + stone_size * 0.5;
    draw_filled_circle_mut(
        &mut img,
        (ctr_x as i32, ctr_y as i32),
        (stone_size * 0.25) as i32,
        Rgba([0, 255, 0, 255]),
    );

    // Draw a red circle in the top right corner for orientation
    let ctr_x = origin_x + (config.board.height as f32 - 0.5) * stone_size;
    let ctr_y = origin_y + stone_size * 0.5;
    draw_filled_circle_mut(
        &mut img,
        (ctr_x as i32, ctr_y as i32),
        (stone_size * 0.25) as i32,
        Rgba([255, 0, 0, 255]),
    );

    img
}

/// Returns the projection matrix that maps the display to the screen.
fn get_display_projection(config: &Config) -> Projection {
    let stone_size = stone_size(config);
    let ctr = Projection::translate(
        config.display.image_width as f32 * -0.5,
        config.display.image_height as f32 * -0.5,
    );
    let perspective = Projection::from_matrix([
        1.0,
        0.0,
        0.0,
        0.0,
        1.0,
        0.0,
        config.display.perspective_x / (stone_size * config.board.width as f32),
        config.display.perspective_y / (stone_size * config.board.height as f32),
        1.0,
    ])
    .unwrap_or(Projection::scale(1.0, 1.0));
    let scale = Projection::scale(config.display.width, config.display.height);
    let translation_scale =
        u32::max(config.display.image_width, config.display.image_height) as f32 * 0.5;
    let translate = Projection::translate(
        config.display.x * translation_scale,
        config.display.y * translation_scale,
    );
    let rotate = Projection::rotate(config.display.angle.to_radians());
    ctr.and_then(perspective)
        .and_then(scale)
        .and_then(translate)
        .and_then(rotate)
        .and_then(ctr.invert())
}

/// Helper function for calculating the size of a stone on the display.
fn stone_size(config: &Config) -> f32 {
    f32::min(
        config.display.image_width as f32 / config.board.width as f32,
        config.display.image_height as f32 / config.board.height as f32,
    )
}

/// Tries to start capturing from a camera based on the configuration.
fn start_camera(config: &CameraConfig) -> Option<Camera> {
    // Try to find a camera with the given name
    let cameras = nokhwa::query(ApiBackend::Auto).ok()?;
    let camera_info = cameras
        .into_iter()
        .find(|camera| camera.human_name() == config.device)?;

    // Create the camera with default/arbitrary settings (mainly to have it choose a frame format)
    let mut camera = Camera::new(
        camera_info.index().clone(),
        RequestedFormat::new::<RgbFormat>(RequestedFormatType::None),
    )
    .ok()?;

    // Request a specific resolution from the camera, without changing the frame format
    camera
        .set_camera_requset(RequestedFormat::new::<RgbFormat>(
            RequestedFormatType::Closest(CameraFormat::new(
                Resolution::new(config.width, config.height),
                camera.frame_format(),
                10,
            )),
        ))
        .ok()?;

    // Start capturing from the camera
    camera.open_stream().ok()?;
    Some(camera)
}

/// Tries to read a frame from the current camera.
fn read_frame(camera: Option<&mut Camera>) -> Option<RgbImage> {
    camera?.frame().ok()?.decode_image::<RgbFormat>().ok()
}
