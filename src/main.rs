use std::sync::{Arc, RwLock};

use app::{AppState, DisplayCalibration};
use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::IntoResponse,
    routing::{get, put},
    Json, Router,
};
use tokio::net::TcpListener;
use tower_http::services::ServeDir;

mod app;

#[tokio::main]
async fn main() {
    let state = AppState::start();
    let app = Router::new()
        .nest_service("/", ServeDir::new("html"))
        .route("/ws/display", get(websocket_display_handler))
        .route("/api/calibrate/display", put(calibrate_display))
        .with_state(state);

    let listener = TcpListener::bind("0.0.0.0:5410").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn websocket_display_handler(
    State(state): State<Arc<RwLock<AppState>>>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| websocket_display(state, socket))
}

/// Watches for display updates and sends them to the client.
async fn websocket_display(state: Arc<RwLock<AppState>>, mut socket: WebSocket) {
    // Subscribe to display updates
    let mut receiver = state.read().unwrap().subscribe_to_image_broadcast();
    receiver.mark_changed();

    loop {
        // Wait for a new frame to be available
        match receiver.changed().await {
            Ok(()) => {
                // If a new frame is available, send it to the client
                let image_data = receiver.borrow_and_update().clone();
                let msg = Message::Binary(image_data);
                match socket.send(msg).await {
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

/// Updates the display calibration settings.
async fn calibrate_display(
    State(state): State<Arc<RwLock<AppState>>>,
    Json(calibration): Json<DisplayCalibration>,
) -> impl IntoResponse {
    let mut state = state.write().unwrap();
    state.set_display_calibration(calibration);
    "OK"
}
