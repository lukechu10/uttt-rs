//! Data structures for representing the state of the game.

pub enum Player {
    X,
    O,
}

pub enum HasWinner {
    Yes,
    Tie,
    InProgress,
}

/// Representation of the Ultimate-TicTacToe game board.
pub struct Board {
    pub sub_wins: SubBoard,
    pub board: [SubBoard; 9],
    pub player_to_move: Player,
}

impl Default for Board {
    fn default() -> Self {
        Self {
            sub_wins: SubBoard::default(),
            board: [SubBoard::default(); 9],
            // Player X always starts.
            player_to_move: Player::X,
        }
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub struct SubBoard {
    pub x: BitBoard,
    pub o: BitBoard,
}

/// A `u16` bit board.
///
/// Only the first 9 bits are used for representing the board state.
/// `0` represents an empty cell, `1` represents an X.
///
/// The remaining bits are unused and should always be `0`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct BitBoard(pub u16);

impl Default for BitBoard {
    /// Default for a `BitBoard` is `0` (all cells are empty).
    fn default() -> Self {
        Self(0)
    }
}

impl BitBoard {
    /// Check if the bit board has a winning configuration.
    /// This is done by checking if the bit board matches one of the winning patterns.
    pub fn has_winner(self) -> HasWinner {
        const WIN_CONFIGURATIONS: [u16; 8] = [
            0b111000000,
            0b000111000,
            0b000000111,
            0b100100100,
            0b010010010,
            0b001001001,
            0b100010001,
            0b001010100,
        ];

        // Check for win.
        for win_config in WIN_CONFIGURATIONS.into_iter() {
            if self.0 & win_config == win_config {
                return HasWinner::Yes;
            }
        }
        // Check for tie.
        if self.0 == 0b111111111 {
            return HasWinner::Tie;
        }
        HasWinner::InProgress
    }
}

/// Represents a position on the board. Does not store the player who applies the move.
pub struct Move {
    /// The major index (position of the sub-board) of the move.
    /// Range can be assumed to be between 0 and 8 inclusive.
    pub major: u32,
    /// The minor index (position of the cell within a sub-board) of the move.
    /// Range can be assumed to be between 0 and 8 inclusive.
    pub minor: u32,
}
