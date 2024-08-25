use std::sync::{Arc, RwLock};

use config::{Config, DisplayConfig};
use image::{ImageBuffer, Rgba, RgbaImage};
use imageproc::{
    drawing::draw_filled_circle_mut,
    geometric_transformations::{warp, Interpolation, Projection},
};
use tokio::sync::watch;

pub mod config;

/// The global state of the application.
pub struct AppState {
    config: Config,
    frame_ready: watch::Sender<()>,
    image_broadcast: watch::Sender<(u32, u32, Vec<u8>)>,
}

impl AppState {
    /// Starts a new instance of the application.
    pub fn start() -> Arc<RwLock<Self>> {
        let config = Config::default();
        let blank_frame = RgbaImage::new(config.display.image_width, config.display.image_height);
        let (frame_ready, _) = watch::channel(());
        let (image_broadcast, _) = watch::channel(get_frame(blank_frame));
        let state = Self {
            config: Config::default(),
            frame_ready,
            image_broadcast,
        };
        let state_ref = Arc::new(RwLock::new(state));
        Self::spawn_render_loop(state_ref.clone());
        state_ref
    }

    /// Returns a new receiver for the image broadcast channel.
    pub fn subscribe_to_image_broadcast(&self) -> watch::Receiver<(u32, u32, Vec<u8>)> {
        self.image_broadcast.subscribe()
    }

    /// Gets the current display configuration.
    pub fn get_display_config(&self) -> DisplayConfig {
        self.config.display
    }

    /// Sets the display configuration.
    pub fn set_display_config(&mut self, display: DisplayConfig) {
        if display.image_width == 0 || display.image_height == 0 {
            return;
        }

        self.config.display = display;
        self.frame_ready.send_replace(());
    }

    /// Spawns the renderer in a background task.
    fn spawn_render_loop(state_ref: Arc<RwLock<AppState>>) {
        tokio::spawn(async move {
            // Subscribe to state updates
            let mut receiver = state_ref.read().unwrap().frame_ready.subscribe();
            receiver.mark_changed();

            // Wait for the state to update
            loop {
                match receiver.changed().await {
                    Ok(()) => {
                        // If the state changes, rerender the display
                        let state = state_ref.read().unwrap();
                        state
                            .image_broadcast
                            .send_replace(get_frame(render(&state.config)));
                    }
                    Err(_) => {
                        // If the channel is closed, stop rendering
                        return;
                    }
                }
            }
        });
    }
}

/// Renders the display.
fn render(config: &Config) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let raw = render_raw(config);
    let proj = get_display_projection(config);
    warp(&raw, &proj, Interpolation::Bilinear, Rgba([0, 0, 0, 0]))
}

/// Renders the display in a normalized position.
/// This will later be warped according to the display configuration.
fn render_raw(config: &Config) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
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

    // Draw a red circle in the upper right corner for orientation
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

/// Helper method for getting the image data to broadcast.
fn get_frame<P: image::Pixel, Container: std::ops::Deref<Target = [P::Subpixel]>>(
    image: ImageBuffer<P, Container>,
) -> (u32, u32, Container) {
    (image.width(), image.height(), image.into_raw())
}
