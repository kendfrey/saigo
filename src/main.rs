use std::{future::Future, io::Cursor, sync::Arc};

use app::{
    config::{self, BoardConfig, CameraConfig, Config, DisplayConfig},
    AppState,
};
use axum::{
    extract::{
        ws::{Message, WebSocket},
        Query, State, WebSocketUpgrade,
    },
    http::header,
    response::{IntoResponse, Result},
    routing::{get, post, MethodRouter},
    Json, Router,
};
use error::SaigoError;
use image::{buffer::ConvertBuffer, ImageFormat, RgbImage, RgbaImage};
use nokhwa::utils::ApiBackend;
use serde::Deserialize;
use tokio::{net::TcpListener, sync::RwLock};
use tower_http::services::ServeDir;

mod app;
mod error;

#[tokio::main]
async fn main() {
    nokhwa::nokhwa_initialize(|_| {});
    let state = AppState::start();
    let app = Router::new()
        .nest_service("/", ServeDir::new("html"))
        .route("/ws/display", websocket(websocket_display))
        .route("/ws/camera", websocket(websocket_camera))
        .route("/api/config/profiles", get(get_config_profiles))
        .route("/api/config/save", post(post_config_save))
        .route("/api/config/load", post(post_config_load))
        .route("/api/config/delete", post(post_config_delete))
        .route(
            "/api/config/board",
            get(get_config_board).put(put_config_board),
        )
        .route(
            "/api/config/display",
            get(get_config_display).put(put_config_display),
        )
        .route(
            "/api/config/camera",
            get(get_config_camera).put(put_config_camera),
        )
        .route("/api/cameras", get(get_cameras))
        .route(
            "/api/config/camera/reference",
            post(post_camera_config_reference),
        )
        .with_state(state);

    let listener = TcpListener::bind("0.0.0.0:5410").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

/// Watches for display updates and sends them to the client.
async fn websocket_display(state: Arc<RwLock<AppState>>, mut socket: WebSocket) {
    // Subscribe to display updates
    let mut receiver = state.read().await.subscribe_to_display_broadcast();
    receiver.mark_changed();

    loop {
        // Wait for a new frame to be available
        match receiver.changed().await {
            Ok(()) => {
                // If a new frame is available, send it to the client
                let image = receiver.borrow_and_update().clone();
                let mut data = vec![];
                data.extend(image.width().to_be_bytes()); // The first 4 bytes are the width
                data.extend(image.height().to_be_bytes()); // The next 4 bytes are the height
                data.extend(image.into_raw()); // The rest is the image data
                match socket.send(Message::Binary(data)).await {
                    Ok(()) => {}
                    Err(_) => {
                        // If the message fails to send, close the socket
                        return;
                    }
                }
            }
            Err(_) => {
                // If the image broadcast channel is closed, close the socket
                return;
            }
        }
    }
}

/// Watches for camera frames and sends them to the client.
async fn websocket_camera(state: Arc<RwLock<AppState>>, mut socket: WebSocket) {
    // Subscribe to camera frames
    let mut receiver = state.read().await.subscribe_to_camera_broadcast();

    // Lock the board configuration
    let _board_config_lock = state.read().await.lock_board_config().await;

    loop {
        // Wait for a new frame to be available
        match receiver.changed().await {
            Ok(()) => {
                // If a new frame is available, send it to the client
                let image: RgbaImage = receiver.borrow_and_update().convert();
                let mut data = vec![];
                data.extend(image.width().to_be_bytes()); // The first 4 bytes are the width
                data.extend(image.height().to_be_bytes()); // The next 4 bytes are the height
                data.extend(image.into_raw()); // The rest is the image data
                match socket.send(Message::Binary(data)).await {
                    Ok(()) => {}
                    Err(_) => {
                        // If the message fails to send, close the socket
                        return;
                    }
                }
            }
            Err(_) => {
                // If the camera broadcast channel is closed, close the socket
                return;
            }
        }
    }
}

/// Gets the list of available configuration profiles.
async fn get_config_profiles() -> Result<Json<Vec<String>>> {
    let profiles = config::get_profiles()?;
    Ok(Json(profiles))
}

/// Used for deserializing a profile name from a query parameter.
#[derive(Deserialize)]
struct Profile {
    profile: String,
}

/// Saves the current configuration to the specified profile.
async fn post_config_save(
    State(state): State<Arc<RwLock<AppState>>>,
    Query(Profile { profile }): Query<Profile>,
) -> Result<()> {
    state.write().await.save_config(&profile)?;
    Ok(())
}

/// Loads the configuration from the specified profile.
async fn post_config_load(
    State(state): State<Arc<RwLock<AppState>>>,
    Query(Profile { profile }): Query<Profile>,
) -> Result<()> {
    state.write().await.load_config(&profile)?;
    Ok(())
}

/// Deletes the specified profile.
async fn post_config_delete(Query(Profile { profile }): Query<Profile>) -> Result<()> {
    Config::delete(&profile)?;
    Ok(())
}

/// Gets the current board configuration.
async fn get_config_board(State(state): State<Arc<RwLock<AppState>>>) -> Json<BoardConfig> {
    Json(state.read().await.get_board_config().clone())
}

/// Updates the board configuration.
async fn put_config_board(
    State(state): State<Arc<RwLock<AppState>>>,
    Json(board): Json<BoardConfig>,
) -> Result<()> {
    state.write().await.set_board_config(board)?;
    Ok(())
}

/// Gets the current display configuration.
async fn get_config_display(State(state): State<Arc<RwLock<AppState>>>) -> Json<DisplayConfig> {
    Json(state.read().await.get_display_config().clone())
}

/// Updates the display configuration.
async fn put_config_display(
    State(state): State<Arc<RwLock<AppState>>>,
    Json(display): Json<DisplayConfig>,
) -> Result<()> {
    state.write().await.set_display_config(display)?;
    Ok(())
}

/// Gets the current camera configuration.
async fn get_config_camera(State(state): State<Arc<RwLock<AppState>>>) -> Json<CameraConfig> {
    Json(state.read().await.get_camera_config().clone())
}

/// Updates the camera configuration.
async fn put_config_camera(
    State(state): State<Arc<RwLock<AppState>>>,
    Json(camera): Json<CameraConfig>,
) -> Result<()> {
    state.write().await.set_camera_config(camera)?;
    Ok(())
}

/// Gets a list of available cameras.
async fn get_cameras() -> Result<Json<Vec<String>>> {
    let cameras = nokhwa::query(ApiBackend::Auto).map_err(SaigoError::Nokhwa)?;
    let names = cameras.iter().map(|camera| camera.human_name()).collect();
    Ok(Json(names))
}

/// Used for deserializing the query string for [`post_camera_config_reference`].
#[derive(Deserialize)]
struct Take {
    take: bool,
}

/// Gets the current reference image, optionally updating it to the current camera view.
async fn post_camera_config_reference(
    State(state): State<Arc<RwLock<AppState>>>,
    Query(Take { take }): Query<Take>,
) -> Result<impl IntoResponse> {
    if take {
        state.write().await.take_reference_image()?;
    }

    // Encode the result as a PNG image
    let image = state
        .read()
        .await
        .get_camera_config()
        .reference_image
        .clone()
        .unwrap_or(RgbImage::new(1, 1));
    let mut writer = Cursor::new(vec![]);
    image
        .write_to(&mut writer, ImageFormat::Png)
        .map_err(SaigoError::Image)?;
    Ok(([(header::CONTENT_TYPE, "image/png")], writer.into_inner()))
}

/// Helper function for creating a WebSocket route.
fn websocket<Fut>(
    handler: impl Fn(Arc<RwLock<AppState>>, WebSocket) -> Fut + Clone + Send + 'static,
) -> MethodRouter<Arc<RwLock<AppState>>>
where
    Fut: Future<Output = ()> + Send + 'static,
{
    get(
        |State(state): State<Arc<RwLock<AppState>>>, ws: WebSocketUpgrade| async {
            ws.on_upgrade(move |socket| handler(state, socket))
        },
    )
}
