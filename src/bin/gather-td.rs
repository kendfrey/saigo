use std::{thread::sleep, time::Duration};

use saigo::ControlMessage;
use tungstenite::{connect, Message};

fn main() {
    let (mut socket, _) = connect("ws://localhost:5410/ws/control").unwrap();

    loop {
        let json = serde_json::to_string(&ControlMessage::NewTrainingPattern).unwrap();
        _ = socket.send(Message::Text(json));
        sleep(Duration::from_secs(1));
    }
}
