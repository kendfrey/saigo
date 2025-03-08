use std::{
    collections::HashMap,
    io::{BufRead, BufReader, stdin},
    net::TcpStream,
    sync::LazyLock,
};

use regex::Regex;
use saigo::{ControlMessage, GameMessage, SerializableColor, SgfCoord};
use tungstenite::{Message, WebSocket, connect, stream::MaybeTlsStream};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut gtp = Gtp::<MyState>::new();
    gtp.add_command("protocol_version", |_, _| Ok("2".to_string()));
    gtp.add_command("name", |_, _| Ok("Saigo".to_string()));
    gtp.add_command("version", |_, _| Ok(env!("CARGO_PKG_VERSION").to_string()));
    gtp.add_command("known_command", |state, args| {
        if args.is_empty() {
            return Err("Expected command name".to_string());
        }
        let command = &args[0];
        let result = state.gtp.commands.contains_key(command.as_str());
        Ok(result.to_string())
    });
    gtp.add_command("list_commands", |state, _| {
        let commands = state
            .gtp
            .commands
            .keys()
            .copied()
            .collect::<Vec<_>>()
            .join("\n");
        Ok(commands)
    });
    gtp.add_command("quit", |state, _| {
        state.should_quit = true;
        Ok("".to_string())
    });
    gtp.add_command("boardsize", |state, args| {
        state.board_size = args
            .first()
            .ok_or("syntax error")?
            .parse()
            .map_err(|e| format!("{}", e))?;
        Ok("".to_string())
    });
    gtp.add_command("clear_board", |state, _| {
        // Since GTP doesn't tell us directly what color the user is,
        // when a new game is started with the clear_board command,
        // we have to wait for either a genmove or a play command to determine the user's color.
        state.active_user_color = None;
        Ok("".to_string())
    });
    gtp.add_command("komi", |_, _| Ok("".to_string()));
    gtp.add_command("play", |state, args| {
        if state.active_user_color.is_none() {
            state.active_user_color = Some(SerializableColor::White);
            state.send(ControlMessage::NewGame {
                user_color: SerializableColor::White,
            })?;
        }

        let coord = args.get(1).ok_or("syntax error")?;
        let color = SerializableColor::try_from(args[0].as_str())?;
        if color != !state.active_user_color.unwrap() {
            return Err("illegal move".to_string());
        }

        if coord.to_uppercase() == "PASS" {
            state.send(ControlMessage::PlayPass)?;
        } else {
            let coord = SgfCoord::from_gtp_coord(coord, state.board_size)?;
            state.send(ControlMessage::PlayMove { location: coord })?;
        }

        Ok("".to_string())
    });
    gtp.add_command("genmove", |state, args| {
        if state.active_user_color.is_none() {
            state.active_user_color = Some(SerializableColor::Black);
            state.send(ControlMessage::NewGame {
                user_color: SerializableColor::Black,
            })?;
        }

        let color = SerializableColor::try_from(args.first().ok_or("syntax error")?.as_str())?;
        if color != state.active_user_color.unwrap() {
            return Err("wrong color".to_string());
        }

        let coord = match state.read()? {
            GameMessage::Move { location } => location.to_gtp_coord(state.board_size)?,
            GameMessage::Pass => "pass".to_string(),
            GameMessage::Resign => "resign".to_string(),
        };

        Ok(coord)
    });

    let (control_socket, _) = connect("ws://localhost:5410/ws/control").unwrap();
    let (game_socket, _) = connect("ws://localhost:5410/ws/game").unwrap();

    let mut state = MyState::new(&gtp, control_socket, game_socket);

    let lines = BufReader::new(stdin()).lines();
    for line in lines {
        let line = line?;
        gtp.handle_input(&line, &mut state);
        if state.should_quit {
            break;
        }
    }
    Ok(())
}

/// The mutable state accessible to GTP commands.
struct MyState<'a> {
    gtp: &'a Gtp<MyState<'a>>,
    should_quit: bool,
    board_size: u8,
    active_user_color: Option<SerializableColor>,
    control_socket: WebSocket<MaybeTlsStream<TcpStream>>,
    game_socket: WebSocket<MaybeTlsStream<TcpStream>>,
}

impl<'a> MyState<'a> {
    fn new(
        gtp: &'a Gtp<MyState<'a>>,
        control_socket: WebSocket<MaybeTlsStream<TcpStream>>,
        game_socket: WebSocket<MaybeTlsStream<TcpStream>>,
    ) -> Self {
        Self {
            gtp,
            should_quit: false,
            board_size: 19,
            active_user_color: None,
            control_socket,
            game_socket,
        }
    }

    /// Sends a message to the control websocket.
    fn send(&mut self, message: ControlMessage) -> Result<(), String> {
        self.control_socket
            .send(Message::Text(
                serde_json::to_string(&message).map_err(|e| format!("{}", e))?,
            ))
            .map_err(|e| format!("{}", e))
    }

    /// Reads a message from the game websocket.
    fn read(&mut self) -> Result<GameMessage, String> {
        let Message::Text(message) = self.game_socket.read().map_err(|e| format!("{}", e))? else {
            return self.read();
        };
        serde_json::from_str(&message).map_err(|e| format!("{}", e))
    }
}

type GtpCommand<S> = fn(&mut S, &[String]) -> Result<String, String>;

/// The GTP engine.
struct Gtp<S> {
    commands: HashMap<&'static str, GtpCommand<S>>,
}

static COMMAND_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(\d*)\s*(\S*)\s*(.*)$").unwrap());

impl<S> Gtp<S> {
    /// Creates a new GTP engine.
    fn new() -> Self {
        Self {
            commands: HashMap::new(),
        }
    }

    /// Adds a command to the GTP engine.
    fn add_command(&mut self, name: &'static str, command: GtpCommand<S>) {
        self.commands.insert(name, command);
    }

    /// Handles a line of input.
    fn handle_input(&self, line: &str, state: &mut S) {
        let line = line.trim();
        if line.is_empty() {
            return;
        }

        let (id, result) = self.execute(line, state);
        match result {
            Ok(message) => print!("={} {}\n\n", id, message),
            Err(message) => print!("?{} {}\n\n", id, message),
        }
    }

    /// Executes a line of input, returning success or failure.
    fn execute<'a>(&self, line: &'a str, state: &mut S) -> (&'a str, Result<String, String>) {
        let (_, [id, command_name, args]) = COMMAND_REGEX.captures(line).unwrap().extract();
        let args: Vec<String> = args.split_whitespace().map(String::from).collect();
        let Some(command) = self.commands.get(command_name) else {
            return (id, Err("unknown command".to_string()));
        };
        let result = command(state, &args);
        (id, result)
    }
}
