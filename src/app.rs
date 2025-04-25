use std::{mem::take, sync::Arc, time::Duration};

use config::{BoardConfig, CameraConfig, Config, DisplayConfig};
use game::{BoardUpdate, GameState};
use goban::pieces::{goban::Goban, stones::Color, util::coord::Coord};
use image::{Rgb, Rgb32FImage, RgbImage, Rgba, RgbaImage, buffer::ConvertBuffer};
use imageproc::{
    drawing::{draw_filled_circle_mut, draw_filled_rect_mut, draw_polygon_mut},
    geometric_transformations::{Interpolation, Projection, warp, warp_into},
    point::Point,
    rect::Rect,
};
use nokhwa::{
    Camera,
    pixel_format::RgbFormat,
    utils::{ApiBackend, CameraFormat, RequestedFormat, RequestedFormatType, Resolution},
};
use rand::{Rng, SeedableRng, rngs::StdRng};
use saigo::{
    Move, PlayerMove, STONE_SIZE, SerializableColor,
    vision_model::{VisionModel, read_tensor},
};
use tch::{
    Device, Kind, Tensor,
    nn::{self, Module},
};
use tokio::{
    sync::{OwnedRwLockReadGuard, OwnedRwLockWriteGuard, RwLock, broadcast, watch},
    task::{self, JoinHandle},
    time::{self, MissedTickBehavior},
};
use tokio_util::sync::CancellationToken;

use crate::{
    error::SaigoError,
    sync::{OwnedSender, SenderLock},
};

pub mod config;
pub mod game;

type VisionModelOutput = (f32, f32, f32, f32);

/// The global state of the application.
pub struct AppState {
    config: Config,
    /// A proxy lock for client connections to signal that the board configuration should not be changed.
    board_config_lock: Arc<RwLock<()>>,
    display_state: Arc<SenderLock<DisplayState>>,
    display_dirty: watch::Sender<()>,
    display_broadcast: watch::Sender<RgbaImage>,
    camera_dirty: watch::Sender<()>,
    camera_broadcast: watch::Sender<RgbImage>,
    board_camera_broadcast: watch::Sender<RgbImage>,
    raw_board_broadcast: watch::Sender<Vec<Vec<VisionModelOutput>>>,
    board_broadcast: watch::Sender<Goban>,
    game_broadcast: broadcast::Sender<PlayerMove>,
    cancel: CancellationToken,
    background_tasks: Vec<JoinHandle<()>>,
    pub game: Option<GameState>,
    troublesome_points: Vec<Vec<u8>>,
}

impl AppState {
    /// Starts a new instance of the application.
    pub fn start() -> Arc<RwLock<Self>> {
        let config = Config::load(None).expect("Failed to load configuration");
        let width = config.board.width.get();
        let height = config.board.height.get();
        let (display_state, _) = watch::channel(DisplayState::default());
        let (display_dirty, _) = watch::channel(());
        let (display_broadcast, _) = watch::channel(RgbaImage::new(160, 120));
        let (camera_dirty, _) = watch::channel(());
        let (camera_broadcast, _) = watch::channel(RgbImage::new(160, 120));
        let (board_camera_broadcast, _) =
            watch::channel(RgbImage::new(width * STONE_SIZE, height * STONE_SIZE));
        let (raw_board_broadcast, _) =
            watch::channel(vec![
                vec![(0.0, 0.0, 0.0, 1.0); width as usize];
                height as usize
            ]);
        let (board_broadcast, _) = watch::channel(Goban::new((height as u8, width as u8)));
        let (game_broadcast, _) = broadcast::channel(4);
        let state = Self {
            config,
            board_config_lock: Arc::new(RwLock::new(())),
            display_state: Arc::new(SenderLock::new(display_state)),
            display_dirty,
            display_broadcast,
            camera_dirty,
            camera_broadcast,
            board_camera_broadcast,
            raw_board_broadcast,
            board_broadcast,
            game_broadcast,
            cancel: CancellationToken::default(),
            background_tasks: vec![],
            game: None,
            troublesome_points: vec![vec![0u8; width as usize]; height as usize],
        };
        let state_ref = Arc::new(RwLock::new(state));
        Self::start_background_tasks(&state_ref);
        state_ref
    }

    /// Starts the background tasks.
    fn start_background_tasks(state_ref: &Arc<RwLock<Self>>) {
        // This needs to be blocking because it happens during drop
        let mut state = task::block_in_place(|| state_ref.blocking_write());
        let cancel = CancellationToken::new();
        state
            .background_tasks
            .push(Self::spawn_render_loop(state_ref.clone(), cancel.clone()));
        state
            .background_tasks
            .push(Self::spawn_camera_loop(state_ref.clone(), cancel.clone()));
        state.background_tasks.push(Self::spawn_board_vision_loop(
            state_ref.clone(),
            cancel.clone(),
        ));
        state.cancel = cancel;
    }

    /// Stops all background tasks.
    async fn stop_background_tasks(state_ref: &Arc<RwLock<Self>>) -> RestartBackgroundTasksOnDrop {
        let cancel;
        let tasks;
        {
            let mut state = state_ref.write().await;
            cancel = state.cancel.clone();
            tasks = take(&mut state.background_tasks);
        }
        cancel.cancel();
        for task in tasks {
            let _ = task.await;
        }
        RestartBackgroundTasksOnDrop(state_ref.clone())
    }

    /// Returns a new receiver for the display broadcast channel.
    pub fn subscribe_to_display_broadcast(&self) -> watch::Receiver<RgbaImage> {
        self.display_broadcast.subscribe()
    }

    /// Returns a new receiver for the camera broadcast channel.
    pub fn subscribe_to_camera_broadcast(&self) -> watch::Receiver<RgbImage> {
        self.camera_broadcast.subscribe()
    }

    /// Returns a new receiver for the board camera broadcast channel.
    pub fn subscribe_to_board_camera_broadcast(&self) -> watch::Receiver<RgbImage> {
        self.board_camera_broadcast.subscribe()
    }

    /// Returns a new receiver for the raw board broadcast channel.
    pub fn subscribe_to_raw_board_broadcast(&self) -> watch::Receiver<Vec<Vec<VisionModelOutput>>> {
        self.raw_board_broadcast.subscribe()
    }

    /// Returns a new receiver for the board broadcast channel.
    pub fn subscribe_to_board_broadcast(&self) -> watch::Receiver<Goban> {
        self.board_broadcast.subscribe()
    }

    /// Returns a new receiver for the game broadcast channel.
    pub fn subscribe_to_game_broadcast(&self) -> broadcast::Receiver<PlayerMove> {
        self.game_broadcast.subscribe()
    }

    /// Saves the current configuration to the specified profile.
    pub fn save_config(&self, profile: &str) -> Result<(), SaigoError> {
        self.config.save(Some(profile), false)
    }

    /// Loads the configuration from the specified profile.
    pub async fn load_config(
        state_ref: &Arc<RwLock<Self>>,
        profile: &str,
    ) -> Result<(), SaigoError> {
        let _guard = Self::write_board_config(state_ref).await?;
        let mut state = state_ref.write().await;
        state.config = Config::load(Some(profile))?;
        state.config.save(None, false)?;
        state.display_dirty.send_replace(());
        state.camera_dirty.send_replace(());
        state.on_board_config_changed();
        Ok(())
    }

    /// Gets the current board configuration.
    pub fn get_board_config(&self) -> &BoardConfig {
        &self.config.board
    }

    /// Sets the board configuration.
    pub async fn set_board_config(
        state_ref: &Arc<RwLock<Self>>,
        board: BoardConfig,
    ) -> Result<(), SaigoError> {
        let _guard = Self::write_board_config(state_ref).await?;
        let mut state = state_ref.write().await;
        if state.config.board.width != board.width || state.config.board.height != board.height {
            state.config.camera.reference_image = None;
        }
        state.config.board = board;
        state.config.save(None, false)?;
        state.on_board_config_changed();
        Ok(())
    }

    /// Locks the board configuration to prevent changes.
    pub async fn lock_board_config(&self) -> OwnedRwLockReadGuard<()> {
        self.board_config_lock.clone().read_owned().await
    }

    /// Checks whether the board configuration is locked and stops the background tasks if not.
    async fn write_board_config(
        state_ref: &Arc<RwLock<Self>>,
    ) -> Result<(OwnedRwLockWriteGuard<()>, RestartBackgroundTasksOnDrop), SaigoError> {
        let guard = state_ref
            .read()
            .await
            .board_config_lock
            .clone()
            .try_write_owned()
            .map_err(|_| {
                SaigoError::Locked("You can't edit the board size while it is in use.".to_string())
            })?;
        let tasks_guard = Self::stop_background_tasks(state_ref).await;
        Ok((guard, tasks_guard))
    }

    /// Resets certain board size-specific data structures.
    fn on_board_config_changed(&mut self) {
        self.game = None;
        self.troublesome_points = vec![
            vec![0u8; self.config.board.width.get() as usize];
            self.config.board.height.get() as usize
        ];
    }

    /// Gets the current display configuration.
    pub fn get_display_config(&self) -> &DisplayConfig {
        &self.config.display
    }

    /// Sets the display configuration.
    pub fn set_display_config(&mut self, display: DisplayConfig) -> Result<(), SaigoError> {
        self.config.display = display;
        self.display_dirty.send_replace(());
        self.config.save_fast()
    }

    /// Gets the current camera configuration.
    pub fn get_camera_config(&self) -> &CameraConfig {
        &self.config.camera
    }

    /// Sets the camera configuration.
    pub fn set_camera_config(&mut self, mut camera: CameraConfig) -> Result<(), SaigoError> {
        // Transfer the old reference image because it's not included in the serialized config
        camera.reference_image = self.config.camera.reference_image.take();

        // If the camera settings change, reset the camera
        let should_reset = self.config.camera.device != camera.device
            || self.config.camera.width != camera.width
            || self.config.camera.height != camera.height;

        self.config.camera = camera;

        if should_reset {
            self.camera_dirty.send_replace(());
        }

        self.config.save_fast()
    }

    /// Captures a reference image of the board.
    pub fn take_reference_image(&mut self) -> Result<(), SaigoError> {
        self.config.camera.reference_image = Some(self.board_camera_broadcast.borrow().clone());
        self.config.save_reference_image(None)
    }

    /// Tries to take control of the display state.
    pub fn try_own_display_state(&self) -> Option<OwnedSender<DisplayState>> {
        self.display_state.clone().try_own()
    }

    /// Transforms the camera image to the normalized board image.
    fn to_board_image(&self, frame: &RgbImage) -> RgbImage {
        // Buffer to copy the board image to
        let mut board_image = RgbImage::new(
            self.config.board.width.get() * STONE_SIZE,
            self.config.board.height.get() * STONE_SIZE,
        );

        // Transform the image coordinates to between 0 and 1, since that's how the control points are represented
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
                    STONE_SIZE as f32 * (self.config.board.width.get() as f32 - 0.5),
                    STONE_SIZE as f32 * 0.5,
                ),
                (
                    STONE_SIZE as f32 * 0.5,
                    STONE_SIZE as f32 * (self.config.board.height.get() as f32 - 0.5),
                ),
                (
                    STONE_SIZE as f32 * (self.config.board.width.get() as f32 - 0.5),
                    STONE_SIZE as f32 * (self.config.board.height.get() as f32 - 0.5),
                ),
            ],
        )
        .unwrap_or(normalize_transform.invert());

        // Warp the camera frame to the board image
        warp_into(
            frame,
            &normalize_transform.and_then(perspective_transform),
            Interpolation::Bilinear,
            Rgb([0, 0, 0]),
            &mut board_image,
        );

        board_image
    }

    /// Starts a new game.
    pub fn new_game(&mut self, user_color: Color) {
        self.game = Some(GameState::new_vs_external(
            self.config.board.width.get() as usize,
            self.config.board.height.get() as usize,
            user_color,
        ));
    }

    /// Spawns the renderer in a background task.
    fn spawn_render_loop(
        state_ref: Arc<RwLock<Self>>,
        cancel: CancellationToken,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            let broadcast;
            let mut display_state_receiver;
            let mut display_dirty_receiver;
            {
                let state = state_ref.read().await;
                broadcast = state.display_broadcast.clone();
                // Subscribe to state updates
                display_state_receiver = state.display_state.subscribe();
                display_state_receiver.mark_changed();
                // Subscribe to display configuration changes
                display_dirty_receiver = state.display_dirty.subscribe();
                display_dirty_receiver.mark_changed();
            }
            let mut interval = time::interval(Duration::from_millis(1000));
            interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
            interval.reset();
            let mut even_tick = false;

            // Wait for the state to update
            while !cancel.is_cancelled() {
                let result = tokio::select! {
                    r = display_state_receiver.changed() => r.map(|_| false),
                    r = display_dirty_receiver.changed() => r.map(|_| false),
                    _ = interval.tick() => Ok(true),
                };
                match result {
                    Ok(tick) => {
                        // If the state changes, rerender the display
                        display_state_receiver.mark_unchanged();
                        display_dirty_receiver.mark_unchanged();
                        if tick {
                            even_tick = !even_tick;
                        }
                        let display_state = *display_state_receiver.borrow();
                        let state = state_ref.read().await;
                        broadcast.send_replace(state.render(display_state, even_tick));
                    }
                    Err(_) => {
                        // If the channel is closed, stop rendering
                        return;
                    }
                }
            }
        })
    }

    /// Spawns the camera capture in a background task.
    fn spawn_camera_loop(
        state_ref: Arc<RwLock<Self>>,
        cancel: CancellationToken,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            let camera_broadcast;
            let board_camera_broadcast;
            let mut dirty_receiver;
            {
                let state = state_ref.read().await;
                camera_broadcast = state.camera_broadcast.clone();
                board_camera_broadcast = state.board_camera_broadcast.clone();
                // Subscribe to camera configuration changes that require a reset
                dirty_receiver = state.camera_dirty.subscribe();
                dirty_receiver.mark_changed();
            }

            let mut camera: Option<Camera> = None;

            // Limit the frame rate to 10 FPS
            let mut interval = time::interval(Duration::from_millis(100));
            interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
            while !cancel.is_cancelled() {
                interval.tick().await;

                // Save work if no one is listening anyway
                if camera_broadcast.is_closed() && board_camera_broadcast.is_closed() {
                    continue;
                }

                // If the camera capture settings change, reset the camera
                if dirty_receiver.has_changed().unwrap() {
                    dirty_receiver.mark_unchanged();
                    camera = start_camera(&state_ref.read().await.config.camera);
                }

                // Try to capture a frame
                if let Some(frame) = read_frame(camera.as_mut()) {
                    let state = state_ref.read().await;
                    let board_frame = state.to_board_image(&frame);
                    // Broadcast the raw frame
                    camera_broadcast.send_replace(frame);
                    // Broadcast the board frame
                    board_camera_broadcast.send_replace(board_frame);
                }
            }
        })
    }

    /// Spawns the board vision in a background task.
    fn spawn_board_vision_loop(
        state_ref: Arc<RwLock<Self>>,
        cancel: CancellationToken,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            let device = Device::cuda_if_available();
            let mut vs = nn::VarStore::new(device);
            let model = VisionModel::new(vs.root());
            vs.load("model.safetensors").unwrap();

            let raw_board_broadcast;
            let board_broadcast;
            let mut board_camera_receiver;
            {
                let state = state_ref.read().await;
                raw_board_broadcast = state.raw_board_broadcast.clone();
                board_broadcast = state.board_broadcast.clone();
                board_camera_receiver = state.board_camera_broadcast.subscribe();
            }

            let mut current_board = board_broadcast.borrow().clone();
            let mut proposed_update = Err(vec![]);

            while let Ok(()) = board_camera_receiver.changed().await {
                if cancel.is_cancelled() {
                    break;
                }
                // Run the neural network on the camera image
                let reference;
                let mut troublesome_points;
                {
                    let state = state_ref.read().await;
                    reference = match &state.config.camera.reference_image {
                        Some(img) => img.convert(),
                        None => continue,
                    };
                    troublesome_points = state.troublesome_points.clone();
                }
                let img = board_camera_receiver.borrow_and_update().convert();
                let result =
                    task::block_in_place(|| run_vision_model(&model, img, reference, device));
                let board = get_board(&result);
                // Broadcast the raw output of the neural network
                raw_board_broadcast.send_replace(result);
                match board {
                    Ok(board) => {
                        // If the board has changed, broadcast it
                        if board != current_board {
                            current_board = board.clone();
                            board_broadcast.send_replace(board);
                            // If the updated board results in a valid change to the state of the game,
                            // wait the specified cooldown before applying it
                            proposed_update = state_ref
                                .read()
                                .await
                                .game
                                .as_ref()
                                .map_or(Err(vec![]), |g| g.check_for_move(&current_board));
                        }
                        match &mut proposed_update {
                            Ok((update, player, cooldown)) => {
                                if *cooldown > 0 {
                                    // Wait for the move to be stable before applying it
                                    *cooldown -= 1;
                                } else {
                                    // Apply the move and broadcast it
                                    let mut state = state_ref.write().await;
                                    if let Some(game) = &mut state.game {
                                        game.apply_update(*update);
                                        let game_move = match *update {
                                            BoardUpdate::Move(coord) => Move::Move {
                                                location: coord.try_into().unwrap(),
                                            },
                                            BoardUpdate::Pass => Move::Pass,
                                            BoardUpdate::Resign => Move::Resign,
                                            BoardUpdate::PendingMovePlayed(coord) => Move::Move {
                                                location: coord.try_into().unwrap(),
                                            },
                                        };
                                        let _ = state.game_broadcast.send(PlayerMove {
                                            move_: game_move,
                                            player: (*player).into(),
                                        });
                                        state.display_dirty.send_replace(());
                                    }
                                    proposed_update = Err(vec![]);
                                }
                            }
                            Err(incorrect_coords) => {
                                // If some stones are out of place, track the incorrect points
                                handle_troublesome_coords(
                                    incorrect_coords,
                                    &mut troublesome_points,
                                );
                            }
                        }
                    }
                    Err(obscured_coords) => {
                        // If the board is obscured in a small area (in case it's not actually obscured but
                        // some stones are out of place), track the obscured points
                        if obscured_coords.len() < 3 {
                            handle_troublesome_coords(&obscured_coords, &mut troublesome_points);
                        }
                    }
                }

                // Decay the troublesome points
                for row in &mut troublesome_points {
                    for point in row {
                        if *point > 0 {
                            *point -= 1;
                        }
                    }
                }

                state_ref.write().await.troublesome_points = troublesome_points;

                /// Marks the specified coordinates as troublesome.
                fn handle_troublesome_coords(
                    coords: &[(u8, u8)],
                    troublesome_points: &mut [Vec<u8>],
                ) {
                    for (x, y) in coords {
                        if troublesome_points[*y as usize][*x as usize] < 20 {
                            troublesome_points[*y as usize][*x as usize] += 3;
                        }
                    }
                }
            }
        })
    }

    /// Renders the display.
    fn render(&self, display_state: DisplayState, even_tick: bool) -> RgbaImage {
        let raw = self.render_raw(display_state, even_tick);
        let proj = self.get_display_projection();
        warp(&raw, &proj, Interpolation::Bilinear, Rgba([0, 0, 0, 0]))
    }

    /// Renders the display in a normalized position.
    /// This will later be warped according to the display configuration.
    fn render_raw(&self, display_state: DisplayState, even_tick: bool) -> RgbaImage {
        let mut ctx = self.create_rendering_context();

        match display_state {
            DisplayState::Calibrate => {
                self.render_calibrate(&mut ctx);
            }
            DisplayState::Training(seed) => {
                self.render_training(seed, &mut ctx);
            }
            DisplayState::Game => {
                self.render_game(&mut ctx, even_tick);
            }
            DisplayState::GameOver(winner) => {
                self.render_endgame(winner, &mut ctx);
            }
        }

        ctx.into_image()
    }

    /// Creates a new rendering context.
    fn create_rendering_context(&self) -> RenderingContext {
        let img = RgbaImage::new(
            self.config.display.image_width.get(),
            self.config.display.image_height.get(),
        );
        let stone_size = self.stone_size();
        let origin_x = (self.config.display.image_width.get() as f32
            - stone_size * (self.config.board.width.get() - 1) as f32)
            * 0.5;
        let origin_y = (self.config.display.image_height.get() as f32
            - stone_size * (self.config.board.height.get() - 1) as f32)
            * 0.5;

        RenderingContext {
            img,
            stone_size,
            origin_x,
            origin_y,
        }
    }

    /// Renders the calibration pattern.
    fn render_calibrate(&self, ctx: &mut RenderingContext) {
        // Draw a dot on every intersection
        for x in 0..self.config.board.width.get() {
            for y in 0..self.config.board.height.get() {
                ctx.fill_circle(x as f32, y as f32, 0.25, Rgba([255, 255, 255, 255]));
            }
        }

        // Draw a green circle in the top left corner for orientation
        ctx.fill_circle(0.0, 0.0, 0.5, Rgba([0, 255, 0, 255]));

        // Draw a red circle in the top right corner for orientation
        ctx.fill_circle(
            self.config.board.width.get() as f32 - 1.0,
            0.0,
            0.5,
            Rgba([255, 0, 0, 255]),
        );
    }

    /// Renders a random pattern for training the neural network.
    fn render_training(&self, seed: <StdRng as SeedableRng>::Seed, ctx: &mut RenderingContext) {
        let mut rng = StdRng::from_seed(seed);

        // Draw random circles on a fraction of the intersections
        for x in 0..self.config.board.width.get() {
            for y in 0..self.config.board.height.get() {
                if rng.random_bool(0.1) {
                    let size = rng.random_range(0.0..1.0);
                    let color = random_color(&mut rng);
                    ctx.fill_circle(x as f32, y as f32, size, color);
                }
            }
        }

        // Draw a random solid triangle
        let x1 = rng.random_range(0.0..self.config.board.width.get() as f32);
        let y1 = rng.random_range(0.0..self.config.board.height.get() as f32);
        let x2 = rng.random_range(0.0..self.config.board.width.get() as f32);
        let y2 = rng.random_range(0.0..self.config.board.height.get() as f32);
        let x3 = rng.random_range(0.0..self.config.board.width.get() as f32);
        let y3 = rng.random_range(0.0..self.config.board.height.get() as f32);
        let color = random_color(&mut rng);

        ctx.fill_polygon(
            &[Point::new(x1, y1), Point::new(x2, y2), Point::new(x3, y3)],
            color,
        );

        fn random_color(rng: &mut StdRng) -> Rgba<u8> {
            if rng.random_bool(0.1) {
                Rgba([255, 255, 255, 255])
            } else {
                Rgba([
                    rng.random_range(0..=255),
                    rng.random_range(0..=255),
                    rng.random_range(0..=255),
                    255,
                ])
            }
        }
    }

    /// Renders the display for the active game.
    fn render_game(&self, ctx: &mut RenderingContext, even_tick: bool) {
        let width = self.config.board.width.get() as f32;
        let height = self.config.board.height.get() as f32;
        let Some(game) = &self.game else {
            return;
        };

        // Draw a white bar on the edge of the board nearest to the current player
        // Draw the bar in yellow if the user must first play the last move
        let bottom_player = if game.user_white && !game.user_black {
            Color::White
        } else {
            Color::Black
        };
        let top = if game.game.turn() == bottom_player {
            height - 1.5
        } else {
            -0.5
        };
        let color = if game.pending_move.is_some() {
            Rgba([127, 127, 0, 255])
        } else {
            Rgba([127, 127, 127, 255])
        };
        ctx.fill_rectangle(-0.5, top, width, 1.0, color);

        // Blink points that the vision model is finding difficult
        if even_tick {
            for y in 0..self.config.board.height.get() {
                for x in 0..self.config.board.width.get() {
                    if self.troublesome_points[y as usize][x as usize] >= 10 {
                        ctx.fill_circle(x as f32, y as f32, 1.5, Rgba([255, 0, 0, 255]));
                    }
                }
            }
        }

        // If the opponent's last move hasn't been played yet, blink its location on the board
        if !even_tick {
            if let Some(coord) = game.pending_move {
                let (x, y) = coord;
                ctx.fill_circle(x as f32, y as f32, 0.75, Rgba([0, 0, 0, 255]));
                ctx.fill_circle(x as f32, y as f32, 0.375, Rgba([255, 255, 255, 255]));
            }
        }
    }

    /// Renders the game over display.
    fn render_endgame(&self, winner: SerializableColor, ctx: &mut RenderingContext) {
        let Some(game) = &self.game else {
            return;
        };

        // Highlight the winner's side of the board in green, and the loser's side in red
        let bottom_player = if game.user_white && !game.user_black {
            SerializableColor::White
        } else {
            SerializableColor::Black
        };
        let (bottom_color, top_color): (Rgba<u8>, Rgba<u8>) = if winner == bottom_player {
            (Rgba([0, 255, 0, 255]), Rgba([255, 0, 0, 255]))
        } else {
            (Rgba([255, 0, 0, 255]), Rgba([0, 255, 0, 255]))
        };
        ctx.fill_rectangle(
            -0.5,
            -0.5 + self.config.board.height.get() as f32 * 0.5,
            self.config.board.width.get() as f32,
            self.config.board.height.get() as f32 * 0.5,
            bottom_color,
        );
        ctx.fill_rectangle(
            -0.5,
            -0.5,
            self.config.board.width.get() as f32,
            self.config.board.height.get() as f32 * 0.5,
            top_color,
        );
    }

    /// Returns the projection matrix that maps the display to the screen.
    fn get_display_projection(&self) -> Projection {
        let stone_size = self.stone_size();
        let ctr = Projection::translate(
            self.config.display.image_width.get() as f32 * -0.5,
            self.config.display.image_height.get() as f32 * -0.5,
        );
        let perspective = Projection::from_matrix([
            1.0,
            0.0,
            0.0,
            0.0,
            1.0,
            0.0,
            self.config.display.perspective_x / (stone_size * self.config.board.width.get() as f32),
            self.config.display.perspective_y
                / (stone_size * self.config.board.height.get() as f32),
            1.0,
        ])
        .unwrap_or(Projection::scale(1.0, 1.0));
        let scale = Projection::scale(self.config.display.width, self.config.display.height);
        let translation_scale = u32::max(
            self.config.display.image_width.get(),
            self.config.display.image_height.get(),
        ) as f32
            * 0.5;
        let translate = Projection::translate(
            self.config.display.x * translation_scale,
            self.config.display.y * translation_scale,
        );
        let rotate = Projection::rotate(self.config.display.angle.to_radians());
        ctr.and_then(perspective)
            .and_then(scale)
            .and_then(translate)
            .and_then(rotate)
            .and_then(ctr.invert())
    }

    /// Helper function for calculating the size of a stone on the display.
    fn stone_size(&self) -> f32 {
        f32::min(
            self.config.display.image_width.get() as f32 / self.config.board.width.get() as f32,
            self.config.display.image_height.get() as f32 / self.config.board.height.get() as f32,
        ) * 0.99
    }
}

/// Adds drop glue to ensure background tasks are restarted after changing the configuration.
struct RestartBackgroundTasksOnDrop(Arc<RwLock<AppState>>);

impl Drop for RestartBackgroundTasksOnDrop {
    fn drop(&mut self) {
        AppState::start_background_tasks(&self.0);
    }
}

/// The pattern or information displayed on the screen.
#[derive(Clone, Copy, Default)]
pub enum DisplayState {
    #[default]
    Calibrate,
    Training(<StdRng as SeedableRng>::Seed),
    Game,
    GameOver(SerializableColor),
}

/// Helper struct for rendering the display.
struct RenderingContext {
    img: RgbaImage,
    stone_size: f32,
    origin_x: f32,
    origin_y: f32,
}

impl RenderingContext {
    /// Draws a filled circle.
    fn fill_circle(&mut self, x: f32, y: f32, size: f32, color: Rgba<u8>) {
        let ctr_x = self.origin_x + x * self.stone_size;
        let ctr_y = self.origin_y + y * self.stone_size;
        draw_filled_circle_mut(
            &mut self.img,
            (ctr_x as i32, ctr_y as i32),
            (self.stone_size * 0.5 * size) as i32,
            color,
        );
    }

    /// Draws a filled rectangle.
    fn fill_rectangle(&mut self, x: f32, y: f32, width: f32, height: f32, color: Rgba<u8>) {
        let left = self.origin_x + x * self.stone_size;
        let top = self.origin_y + y * self.stone_size;
        let right = left + width * self.stone_size;
        let bottom = top + height * self.stone_size;
        let left = left as i32;
        let top = top as i32;
        let right = right as i32;
        let bottom = bottom as i32;
        draw_filled_rect_mut(
            &mut self.img,
            Rect::at(left, top).of_size((right - left) as u32, (bottom - top) as u32),
            color,
        );
    }

    /// Draws a filled polygon.
    fn fill_polygon(&mut self, points: &[Point<f32>], color: Rgba<u8>) {
        let mapped_points = points
            .iter()
            .map(|point| Point {
                x: (self.origin_x + point.x * self.stone_size) as i32,
                y: (self.origin_y + point.y * self.stone_size) as i32,
            })
            .collect::<Vec<_>>();
        draw_polygon_mut(&mut self.img, mapped_points.as_slice(), color);
    }

    /// Converts the rendering context into an image.
    fn into_image(self) -> RgbaImage {
        self.img
    }
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

/// Runs the vision model on an image of the board and returns the state of each intersection.
fn run_vision_model(
    model: &VisionModel,
    image: Rgb32FImage,
    reference: Rgb32FImage,
    device: Device,
) -> Vec<Vec<VisionModelOutput>> {
    let width = reference.width() / STONE_SIZE;
    let height = reference.height() / STONE_SIZE;
    let mut input = Vec::with_capacity((width * height) as usize);
    for y in 0..height {
        for x in 0..width {
            input.push(read_tensor(
                &image,
                &reference,
                x * STONE_SIZE,
                y * STONE_SIZE,
            ));
        }
    }
    let output: Vec<Vec<f32>> = model
        .forward(&Tensor::stack(&input, 0).to(device))
        .softmax(1, Kind::Float)
        .try_into()
        .unwrap();

    let mut result = Vec::with_capacity(height as usize);
    for y in 0..height {
        result.push(Vec::with_capacity(width as usize));
        for x in 0..width {
            let index = (y * width + x) as usize;
            result[y as usize].push((
                output[index][0],
                output[index][1],
                output[index][2],
                output[index][3],
            ));
        }
    }
    result
}

/// Calculates the most likely state of the board, or returns the list of obscured points.
fn get_board(probabilities: &[Vec<VisionModelOutput>]) -> Result<Goban, Vec<Coord>> {
    let mut goban = Goban::new((probabilities.len() as u8, probabilities[0].len() as u8));
    let mut obscured_coords = Vec::new();
    for (y, row) in probabilities.iter().enumerate() {
        for (x, (empty, black, white, _)) in row.iter().enumerate() {
            let coord = (x as u8, y as u8);
            let color = if *empty > 0.5 {
                continue;
            } else if *black > 0.9 {
                Color::Black
            } else if *white > 0.9 {
                Color::White
            } else {
                obscured_coords.push(coord);
                continue;
            };
            goban.push(coord, color);
        }
    }
    if !obscured_coords.is_empty() {
        Err(obscured_coords)
    } else {
        Ok(goban)
    }
}
