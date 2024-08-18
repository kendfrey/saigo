use std::{
    cmp::{max, min},
    sync::{Arc, RwLock},
    vec,
};

use image::{ImageBuffer, Rgba, RgbaImage};
use imageproc::{
    drawing::draw_filled_circle_mut,
    geometric_transformations::{warp, Interpolation, Projection},
};
use serde::Deserialize;
use tokio::sync::watch;

const IMG_WIDTH: u32 = 1280;
const IMG_HEIGHT: u32 = 720;

/// The settings used to render the display.
#[derive(Deserialize)]
pub struct DisplayCalibration {
    board_width: u32,
    board_height: u32,
    angle: f32,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    perspective_x: f32,
    perspective_y: f32,
}

impl Default for DisplayCalibration {
    fn default() -> Self {
        Self {
            board_width: 19,
            board_height: 19,
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

/// The global state of the application.
pub struct AppState {
    display_calibration: DisplayCalibration,
    frame_ready: watch::Sender<()>,
    image_broadcast: watch::Sender<Vec<u8>>,
}

impl AppState {
    /// Starts a new instance of the application.
    pub fn start() -> Arc<RwLock<Self>> {
        let (frame_ready, _) = watch::channel(());
        let (image_broadcast, _) = watch::channel(vec![0; (IMG_WIDTH * IMG_HEIGHT * 4) as usize]);
        let state = Self {
            display_calibration: DisplayCalibration::default(),
            frame_ready,
            image_broadcast,
        };
        let state_ref = Arc::new(RwLock::new(state));
        Self::spawn_render_loop(state_ref.clone());
        state_ref
    }

    /// Returns a new receiver for the image broadcast channel.
    pub fn subscribe_to_image_broadcast(&self) -> watch::Receiver<Vec<u8>> {
        self.image_broadcast.subscribe()
    }

    /// Sets the display calibration settings.
    pub fn set_display_calibration(&mut self, display_calibration: DisplayCalibration) {
        self.display_calibration = display_calibration;
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
                        let data = state.render().into_raw();
                        state.image_broadcast.send_replace(data);
                    }
                    Err(_) => {
                        // If the channel is closed, stop rendering
                        return;
                    }
                }
            }
        });
    }

    /// Renders the display.
    fn render(&self) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
        let stone_size = min(
            IMG_WIDTH / self.display_calibration.board_width,
            IMG_HEIGHT / self.display_calibration.board_height,
        );
        warp(
            &self.render_raw(stone_size),
            &self.get_display_projection(stone_size),
            Interpolation::Bilinear,
            Rgba([0, 0, 0, 0]),
        )
    }

    /// Renders the display in a normalized position.
    /// This will later be warped according to the display calibration settings.
    fn render_raw(&self, stone_size: u32) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
        let origin_x = (IMG_WIDTH - stone_size * self.display_calibration.board_width) / 2;
        let origin_y = (IMG_HEIGHT - stone_size * self.display_calibration.board_height) / 2;

        // Draw a dot on every intersection
        let mut img = RgbaImage::new(IMG_WIDTH, IMG_HEIGHT);
        for x in 0..self.display_calibration.board_width {
            for y in 0..self.display_calibration.board_height {
                let ctr_x = origin_x + x * stone_size + stone_size / 2;
                let ctr_y = origin_y + y * stone_size + stone_size / 2;
                draw_filled_circle_mut(
                    &mut img,
                    (ctr_x as i32, ctr_y as i32),
                    (stone_size / 8) as i32,
                    Rgba([255, 255, 255, 255]),
                );
            }
        }

        // Draw a red circle in the upper right corner for orientation
        let ctr_x =
            origin_x + (self.display_calibration.board_height - 1) * stone_size + stone_size / 2;
        let ctr_y = origin_y + stone_size / 2;
        draw_filled_circle_mut(
            &mut img,
            (ctr_x as i32, ctr_y as i32),
            (stone_size / 4) as i32,
            Rgba([255, 0, 0, 255]),
        );

        img
    }

    /// Returns the projection matrix that maps the display to the screen.
    fn get_display_projection(&self, stone_size: u32) -> Projection {
        let ctr = Projection::translate(IMG_WIDTH as f32 * -0.5, IMG_HEIGHT as f32 * -0.5);
        let perspective = Projection::from_matrix([
            1.0,
            0.0,
            0.0,
            0.0,
            1.0,
            0.0,
            self.display_calibration.perspective_x
                / (stone_size * self.display_calibration.board_width) as f32,
            self.display_calibration.perspective_y
                / (stone_size * self.display_calibration.board_height) as f32,
            1.0,
        ])
        .unwrap_or(Projection::scale(1.0, 1.0));
        let scale = Projection::scale(
            self.display_calibration.width,
            self.display_calibration.height,
        );
        let translate = Projection::translate(
            self.display_calibration.x * max(IMG_WIDTH, IMG_HEIGHT) as f32 * 0.5,
            self.display_calibration.y * max(IMG_WIDTH, IMG_HEIGHT) as f32 * 0.5,
        );
        let rotate = Projection::rotate(self.display_calibration.angle.to_radians());
        ctr.and_then(perspective)
            .and_then(scale)
            .and_then(translate)
            .and_then(rotate)
            .and_then(ctr.invert())
    }
}
