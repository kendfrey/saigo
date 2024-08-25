use std::{
    future::Future,
    sync::{Arc, RwLock},
};

use app::{config::DisplayConfig, AppState};
use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::IntoResponse,
    routing::{get, MethodRouter},
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
        .route("/ws/display", websocket(websocket_display))
        .route(
            "/api/config/display",
            get(get_config_display).put(put_config_display),
        )
        .with_state(state);

    let listener = TcpListener::bind("0.0.0.0:5410").await.unwrap();
    axum::serve(listener, app).await.unwrap();
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
                let (width, height, image_data) = receiver.borrow_and_update().clone();
                let mut data = vec![];
                data.extend(width.to_be_bytes()); // The first 4 bytes are the width
                data.extend(height.to_be_bytes()); // The next 4 bytes are the height
                data.extend(image_data); // The rest is the image data
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

/// Gets the current display configuration.
async fn get_config_display(State(state): State<Arc<RwLock<AppState>>>) -> impl IntoResponse {
    Json(state.read().unwrap().get_display_config())
}

/// Updates the display configuration.
async fn put_config_display(
    State(state): State<Arc<RwLock<AppState>>>,
    Json(display): Json<DisplayConfig>,
) -> impl IntoResponse {
    state.write().unwrap().set_display_config(display);
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
