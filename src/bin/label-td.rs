use std::path::PathBuf;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::get,
    Router,
};
use clap::Parser;
use serde::Deserialize;
use tokio::{fs, net::TcpListener};
use tokio_stream::{wrappers::ReadDirStream, StreamExt};
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let mut indexed_files: Vec<(String, usize)> =
        ReadDirStream::new(fs::read_dir(&args.data).await?)
            .filter_map(|entry| {
                let path = entry.ok()?.path();
                // Only include PNG files
                if path.extension()? != "png" || !path.is_file() {
                    return None;
                }
                let name = path.file_stem()?.to_string_lossy().to_string();
                // Only include files whose names are numbers
                let index = name.parse::<usize>().ok()?;
                Some((name, index))
            })
            .collect()
            .await;
    // Sort the file names numerically
    indexed_files.sort_unstable_by_key(|(_, index)| *index);
    let files: Vec<String> = indexed_files.into_iter().map(|(name, _)| name).collect();

    if files.is_empty() {
        println!("No training data found in the specified directory.");
        return Ok(());
    }

    let app = Router::new()
        .nest_service("/", ServeDir::new("html/label-td"))
        .route("/api/first", get(get_first))
        .route("/api/previous", get(get_previous))
        .route("/api/next", get(get_next))
        .route("/api/last", get(get_last))
        .route("/api/image", get(get_image))
        .route("/api/labels", get(get_labels).put(put_labels))
        .with_state((args.data, files));

    let listener = TcpListener::bind("0.0.0.0:5416").await.unwrap();
    println!(
        "Listening on http://localhost:{}/",
        listener.local_addr().unwrap().port()
    );
    println!("Press Ctrl+C to exit.");
    axum::serve(listener, app).await.unwrap();
    Ok(())
}

/// Labels training data for the image recognition neural network.
#[derive(Parser)]
struct Args {
    /// The directory containing the data to be labeled.
    data: PathBuf,
}

/// Used for deserializing an index from a query parameter.
#[derive(Deserialize)]
struct Index {
    index: String,
}

/// Returns the index of the first data file.
async fn get_first(State((_, files)): State<(PathBuf, Vec<String>)>) -> String {
    files[0].clone()
}

/// Returns the index of the data file before the specified one.
async fn get_previous(
    State((_, files)): State<(PathBuf, Vec<String>)>,
    Query(Index { index }): Query<Index>,
) -> String {
    let i = files.iter().position(|f| f == &index).unwrap_or(0);
    files[if i == 0 { 0 } else { i - 1 }].clone()
}

/// Returns the index of the data file after the specified one.
async fn get_next(
    State((_, files)): State<(PathBuf, Vec<String>)>,
    Query(Index { index }): Query<Index>,
) -> String {
    let i = files
        .iter()
        .position(|f| f == &index)
        .unwrap_or(files.len() - 1);
    files[if i == files.len() - 1 {
        files.len() - 1
    } else {
        i + 1
    }]
    .clone()
}

/// Returns the index of the last data file.
async fn get_last(State((_, files)): State<(PathBuf, Vec<String>)>) -> String {
    files.last().unwrap().clone()
}

/// Returns the image data for the specified index.
async fn get_image(
    State((data, _)): State<(PathBuf, Vec<String>)>,
    Query(Index { index }): Query<Index>,
) -> Result<Vec<u8>, StatusCode> {
    let path = data.join(format!("{}.png", index));
    match fs::read(path).await {
        Ok(data) => Ok(data),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

/// Returns the labels for the specified index.
async fn get_labels(
    State((data, _)): State<(PathBuf, Vec<String>)>,
    Query(Index { index }): Query<Index>,
) -> Result<String, StatusCode> {
    let path = data.join(format!("{}.txt", index));
    match fs::read_to_string(path).await {
        Ok(labels) => Ok(labels),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

/// Writes the labels for the specified index.
async fn put_labels(
    State((data, _)): State<(PathBuf, Vec<String>)>,
    Query(Index { index }): Query<Index>,
    labels: String,
) -> Result<(), StatusCode> {
    let path = data.join(format!("{}.txt", index));
    match fs::write(path, labels).await {
        Ok(_) => Ok(()),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
