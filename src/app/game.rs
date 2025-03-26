use goban::{
    pieces::{goban::Goban, stones::Color, util::coord::Coord},
    rules::{GobanSizes, JAPANESE, Move, game::Game},
};

/// An in-progress game.
pub struct GameState {
    pub user_black: bool,
    pub user_white: bool,
    pub game: Game,
    pub pending_move: Option<Coord>,
}

/// An update to the game state.
#[derive(Clone, Copy)]
pub enum BoardUpdate {
    Move(Coord),
    Pass,
    Resign,
    PendingMovePlayed(Coord),
}

impl GameState {
    /// Starts a new game.
    pub fn new_vs_external(width: usize, height: usize, user_color: Color) -> Self {
        Self {
            user_black: user_color == Color::Black,
            user_white: user_color == Color::White,
            game: Game::new(GobanSizes::Custom(width, height), JAPANESE),
            pending_move: None,
        }
    }

    /// Checks whether the user has made a move on the physical board, and if so,
    /// returns the corresponding action and a cooldown to wait before committing to the move.
    pub fn check_for_move(&self, new_board: &Goban) -> Option<(BoardUpdate, Color, u32)> {
        let user_turn = match self.game.turn() {
            Color::Black => self.user_black,
            Color::White => self.user_white,
        };
        if let Some(pending_move) = self.pending_move {
            self.check_for_pending_move(new_board, pending_move)
        } else if user_turn {
            self.check_for_user_move(new_board)
        } else {
            None
        }
    }

    /// Checks whether the user has made a new move on the physical board, and if so,
    /// returns the corresponding action and a cooldown to wait before committing to the move.
    fn check_for_user_move(&self, new_board: &Goban) -> Option<(BoardUpdate, Color, u32)> {
        // Record all stones that weren't on the board as of the last known game state
        let mut new_player_stones = vec![];
        let mut new_opponent_stones = vec![];
        let (width, height) = new_board.size();
        for x in 0..width {
            for y in 0..height {
                let new_stone = new_board.get_color((x, y));
                let old_stone = self.game.goban().get_color((x, y));
                if new_stone == Some(self.game.turn()) && old_stone.is_none() {
                    new_player_stones.push((x, y));
                } else if new_stone == Some(!self.game.turn()) && old_stone.is_none() {
                    new_opponent_stones.push((x, y));
                }
            }
        }
        if new_player_stones.len() == 1
            && is_valid_move(&self.game, new_player_stones[0], new_board)
        {
            // If the user places a single stone of their own color and it's a valid move, return it
            let coord = new_player_stones[0];
            let cooldown =
                if coord.0 == 0 || coord.0 == width - 1 || coord.1 == 0 || coord.1 == height - 1 {
                    10 // Use a longer cooldown for moves on the edge of the board
                } else {
                    2
                };
            Some((BoardUpdate::Move(coord), self.game.turn(), cooldown))
        } else if new_player_stones.len() == 2 && new_opponent_stones.is_empty() {
            // If the user places two stones of their own color, treat it as a pass
            Some((BoardUpdate::Pass, self.game.turn(), 20))
        } else if new_opponent_stones.len() == 2 && new_player_stones.is_empty() {
            // If the user places two stones of the opponent's color, treat it as a resignation
            Some((BoardUpdate::Resign, self.game.turn(), 20))
        } else {
            None
        }
    }

    /// Checks whether the user has placed the pending move on the physical board, and if so,
    /// returns the corresponding action and a cooldown to wait before committing to the move.
    fn check_for_pending_move(
        &self,
        new_board: &Goban,
        pending_move: Coord,
    ) -> Option<(BoardUpdate, Color, u32)> {
        // If the user places an external move, return it.
        if new_board == self.game.goban() {
            Some((
                BoardUpdate::PendingMovePlayed(pending_move),
                !self.game.turn(),
                1,
            ))
        } else {
            None
        }
    }

    /// Applies an update returned by `check_for_move` to the game state.
    pub fn apply_update(&mut self, update: BoardUpdate) {
        match update {
            BoardUpdate::Move(coord) => {
                self.game.play(Move::Play(coord.0, coord.1));
            }
            BoardUpdate::Pass => {
                self.game.play(Move::Pass);
            }
            BoardUpdate::Resign => {}
            BoardUpdate::PendingMovePlayed(_) => {
                self.pending_move = None;
            }
        };
    }

    /// Plays a move from the API and requests the user to play the corresponding move on the physical board.
    pub fn play_external_move(&mut self, coord: Coord) {
        self.game.play(Move::Play(coord.0, coord.1));
        self.pending_move = Some(coord);
    }

    /// Plays a pass from the API.
    pub fn play_external_pass(&mut self) {
        self.game.play(Move::Pass);
        self.pending_move = None;
    }
}

/// Checks whether the specified move in the specified game results in the expected board.
fn is_valid_move(game: &Game, coord: Coord, expected_board: &Goban) -> bool {
    match game.clone().try_play(Move::Play(coord.0, coord.1)) {
        Ok(new_board) => new_board.goban() == expected_board,
        _ => false,
    }
}
