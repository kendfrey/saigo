use std::{
    fs::{self, File},
    path::PathBuf,
    time::{Duration, Instant},
};

use clap::Parser;
use image::{buffer::ConvertBuffer, RgbImage};
use saigo::{deserialize_image, ControlMessage};
use tungstenite::{connect, Message};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    if args.out.try_exists()? {
        println!(
            "Directory {} already exists and will not be overwritten.",
            args.out.display()
        );
        return Ok(());
    }

    let (mut control_socket, _) = connect("ws://localhost:5410/ws/control").unwrap();
    let (mut camera_socket, _) = connect("ws://localhost:5410/ws/board-camera").unwrap();
    let new_training_pattern =
        Message::Text(serde_json::to_string(&ControlMessage::NewTrainingPattern).unwrap());
    _ = control_socket.send(new_training_pattern.clone());

    fs::create_dir_all(&args.out)?;

    reqwest::blocking::Client::new()
        .post("http://localhost:5410/api/config/camera/reference?take=false")
        .send()?
        .error_for_status()?
        .copy_to(&mut File::create_new(args.out.join("reference.png"))?)?;

    let interval = Duration::from_secs(5);
    let mut next_capture = Instant::now() + interval;
    let mut num = 0;
    loop {
        if let Message::Binary(data) = camera_socket.read()? {
            if Instant::now() >= next_capture {
                let image: RgbImage = deserialize_image(data).convert();
                image.save(args.out.join(format!("{}.png", num)))?;
                num += 1;

                next_capture += interval;
                _ = control_socket.send(new_training_pattern.clone());
            }
        }
    }
}

/// Gathers training data for the image recognition neural network.
#[derive(Parser)]
struct Args {
    /// The directory to save the training data to.
    out: PathBuf,
}
