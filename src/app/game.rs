use goban::{
    pieces::{goban::Goban, stones::Color, util::coord::Coord},
    rules::{GobanSizes, JAPANESE, Move, game::Game},
};
use saigo::SgfCoord;

/// An in-progress game.
pub struct GameState {
    user_color: Color,
    game: Game,
    status: Status,
}

/// The current status of the game, including whose turn it is and related information.
#[derive(PartialEq, Eq)]
pub enum Status {
    UserTurn,
    OpponentTurn,
    OpponentPlayed(SgfCoord),
}

/// An update to the game state.
pub enum StateUpdate {
    UserMove(SgfCoord),
    UserPass,
    UserResign,
    OpponentMovePlayed,
    None,
}

impl GameState {
    /// Starts a new game.
    pub fn new(width: usize, height: usize, user_color: Color) -> Self {
        Self {
            user_color,
            game: Game::new(GobanSizes::Custom(width, height), JAPANESE),
            status: match user_color {
                Color::Black => Status::UserTurn,
                Color::White => Status::OpponentTurn,
            },
        }
    }

    /// Returns the user's color.
    pub fn user_color(&self) -> Color {
        self.user_color
    }

    /// Returns the current status.
    pub fn status(&self) -> &Status {
        &self.status
    }

    /// Checks whether the user has made a move on the physical board, and if so,
    /// updates the state and returns the corresponding action.
    pub fn check_for_move(&mut self, new_board: &Goban) -> StateUpdate {
        match &self.status {
            Status::UserTurn => {
                // Record all stones that weren't on the board as of the last known game state
                let mut new_user_stones = vec![];
                let mut new_opponent_stones = vec![];
                let (width, height) = new_board.size();
                for x in 0..width {
                    for y in 0..height {
                        let new_stone = new_board.get_color((x, y));
                        let old_stone = self.game.goban().get_color((x, y));
                        if new_stone == Some(self.user_color) && old_stone.is_none() {
                            new_user_stones.push((x, y));
                        } else if new_stone == Some(!self.user_color) && old_stone.is_none() {
                            new_opponent_stones.push((x, y));
                        }
                    }
                }
                if new_user_stones.len() == 1
                    && is_valid_move(&self.game, new_user_stones[0], new_board)
                {
                    // If the user places a single stone of their own color and it's a valid move, play it
                    let coord = new_user_stones[0];
                    self.game.play(Move::Play(coord.0, coord.1));
                    self.status = Status::OpponentTurn;
                    StateUpdate::UserMove(SgfCoord::try_from(coord).unwrap())
                } else if new_user_stones.len() == 2 && new_opponent_stones.is_empty() {
                    // If the user places two stones of their own color, treat it as a pass
                    self.game.play(Move::Pass);
                    self.status = Status::OpponentTurn;
                    StateUpdate::UserPass
                } else if new_opponent_stones.len() == 2 && new_user_stones.is_empty() {
                    // If the user places two stones of the opponent's color, treat it as a resignation
                    StateUpdate::UserResign
                } else {
                    StateUpdate::None
                }
            }
            Status::OpponentPlayed(sgf_coord) => {
                // If the user places the opponent's last move, update the status to wait for the user's next move
                let coord: Coord = sgf_coord.try_into().unwrap();
                if is_valid_move(&self.game, coord, new_board) {
                    let (x, y) = coord;
                    self.game.play(Move::Play(x, y));
                    self.status = Status::UserTurn;
                    StateUpdate::OpponentMovePlayed
                } else {
                    StateUpdate::None
                }
            }
            Status::OpponentTurn => StateUpdate::None,
        }
    }

    /// Plays a move as the opponent.
    pub fn play_move(&mut self, location: SgfCoord) {
        if self.status != Status::OpponentTurn {
            return;
        }
        self.status = Status::OpponentPlayed(location);
    }

    /// Plays a pass as the opponent.
    pub fn play_pass(&mut self) {
        if self.status != Status::OpponentTurn {
            return;
        }
        self.game.play(Move::Pass);
        self.status = Status::UserTurn;
    }
}

/// Checks whether the specified move in the specified game results in the expected board.
fn is_valid_move(game: &Game, coord: Coord, expected_board: &Goban) -> bool {
    match game.clone().try_play(Move::Play(coord.0, coord.1)) {
        Ok(new_board) => new_board.goban() == expected_board,
        _ => false,
    }
}
