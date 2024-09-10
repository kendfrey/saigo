use std::{
    future::Future,
    io::Cursor,
    sync::{Arc, RwLock},
};

use app::{
    config::{CameraConfig, DisplayConfig},
    AppState,
};
use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    http::{header, StatusCode},
    response::{IntoResponse, Result},
    routing::{get, post, MethodRouter},
    Json, Router,
};
use image::{buffer::ConvertBuffer, ImageFormat, RgbImage, RgbaImage};
use nokhwa::utils::ApiBackend;
use tokio::net::TcpListener;
use tower_http::services::ServeDir;

mod app;

#[tokio::main]
async fn main() {
    nokhwa::nokhwa_initialize(|_| {});
    let state = AppState::start();
    let app = Router::new()
        .nest_service("/", ServeDir::new("html"))
        .route("/ws/display", websocket(websocket_display))
        .route("/ws/camera", websocket(websocket_camera))
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
    let mut receiver = state.read().unwrap().subscribe_to_display_broadcast();
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
    let mut receiver = state.read().unwrap().subscribe_to_camera_broadcast();

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

/// Gets the current display configuration.
async fn get_config_display(State(state): State<Arc<RwLock<AppState>>>) -> Json<DisplayConfig> {
    Json(state.read().unwrap().get_display_config().clone())
}

/// Updates the display configuration.
async fn put_config_display(
    State(state): State<Arc<RwLock<AppState>>>,
    Json(display): Json<DisplayConfig>,
) {
    state.write().unwrap().set_display_config(display);
}

/// Gets the current camera configuration.
async fn get_config_camera(State(state): State<Arc<RwLock<AppState>>>) -> Json<CameraConfig> {
    Json(state.read().unwrap().get_camera_config().clone())
}

/// Updates the camera configuration.
async fn put_config_camera(
    State(state): State<Arc<RwLock<AppState>>>,
    Json(camera): Json<CameraConfig>,
) {
    state.write().unwrap().set_camera_config(camera);
}

/// Gets a list of available cameras.
async fn get_cameras() -> Result<Json<Vec<String>>> {
    let cameras = nokhwa::query(ApiBackend::Auto)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let names = cameras.iter().map(|camera| camera.human_name()).collect();
    Ok(Json(names))
}

/// Gets the current reference image, optionally updating it to the current camera view.
async fn post_camera_config_reference(
    State(state): State<Arc<RwLock<AppState>>>,
    Json(take): Json<bool>,
) -> Result<impl IntoResponse> {
    if take {
        state.write().unwrap().take_reference_image();
    }

    // Encode the result as a PNG image
    let image = state
        .read()
        .unwrap()
        .get_camera_config()
        .reference_image
        .clone()
        .unwrap_or(RgbImage::new(1, 1));
    let mut writer = Cursor::new(vec![]);
    image
        .write_to(&mut writer, ImageFormat::Png)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
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
