//! Data structures for representing the state of the game.

use std::fmt::{self, Display, Formatter};
use std::ops::{BitAnd, BitOr};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Player {
    X,
    O,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HasWinner {
    Yes,
    Tie,
    InProgress,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Winner {
    X,
    O,
    Tie,
    InProgress,
}

/// Representation of the Ultimate-TicTacToe game board.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Board {
    pub sub_wins: WinBoard,
    pub board: [SubBoard; 9],
    pub player_to_move: Player,
    /// The index of the next sub-board. If next player can only move in a specific sub-board, the
    /// value will be in the range of `0..9`. If next player can move anywhere, the value will be
    /// `9`.
    pub next_sub_board: u32,
}

impl Default for Board {
    fn default() -> Self {
        Self {
            sub_wins: WinBoard::default(),
            board: [SubBoard::default(); 9],
            // Player X always starts.
            player_to_move: Player::X,
            // Initially can move anywhere.
            next_sub_board: 9,
        }
    }
}

impl Board {
    /// Create a new [`Board`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the [`Board`] with the applied [`Move`] onto it. This does not change the original
    /// [`Board`]. This method also doesn't check if the move is valid in the context of the game
    /// state.
    ///
    /// Switches the next player to move.
    ///
    /// # Safety
    ///
    /// - `m` must be a valid [`Move`], meaning that the `major` field and the `minor` field must be
    ///   between `0` and `8` inclusive. Any value outside this range will cause undefined behavior.
    #[must_use = "advanced_state_unsafe does not modify original Board"]
    pub unsafe fn advance_state_unsafe(mut self, m: Move) -> Self {
        // SAFETY: range is guaranteed to be valid by the caller. `board` is of length 9 and m.major
        // is in range 0..9.
        let sub_board = self.board.get_unchecked_mut(m.major as usize);

        match self.player_to_move {
            Player::X => {
                sub_board.x = sub_board.x.advance_bitfield_state(m.minor);
                self.player_to_move = Player::O;

                // Update `sub_wins` to keep state in sync.
                // Since we know the major position of the move, we only need to recompute the win
                // state for one of the sub-boards. We also know the player so we only need to
                // re-compute the bitfield of the player.
                if sub_board.x.has_winner() == HasWinner::Yes {
                    self.sub_wins.x.0 |= 1 << m.major
                } else if sub_board.x.0 | sub_board.o.0 == 0b111111111 {
                    self.sub_wins.tie.0 |= 1 << m.major
                }

                // Update `next_sub_board` for next turn.
                // The next sub-board index is the same as the minor index for this turn.
                let sub_wins_or = self.sub_wins.o.0 | self.sub_wins.x.0 | self.sub_wins.tie.0;
                if sub_wins_or & 1 << m.minor != 0 {
                    // The next sub-board has already been won. Next player can move anywhere.
                    self.next_sub_board = 9;
                } else {
                    // The next sub-board has not been won. Next player can only move in this
                    // sub-board.
                    self.next_sub_board = m.minor;
                }
            }
            Player::O => {
                sub_board.o = sub_board.o.advance_bitfield_state(m.minor);
                self.player_to_move = Player::X;

                // Update `sub_wins` to keep state in sync. See above for more details.
                if sub_board.o.has_winner() == HasWinner::Yes {
                    self.sub_wins.o.0 |= 1 << m.major
                } else if sub_board.x.0 | sub_board.o.0 == 0b111111111 {
                    self.sub_wins.tie.0 |= 1 << m.major
                }

                // Update `next_sub_board` for next turn. See above for more details.
                let sub_wins_or = self.sub_wins.o.0 | self.sub_wins.x.0 | self.sub_wins.tie.0;
                if sub_wins_or & 1 << m.minor != 0 {
                    self.next_sub_board = 9;
                } else {
                    self.next_sub_board = m.minor;
                }
            }
        };

        self
    }

    /// Returns the [`Board`] with the applied [`Move`] onto it or `None` if the move is invalid.
    /// This does not change the original [`Board`].
    ///
    /// Switches the next player to move.
    ///
    /// For performance critical code, prefer [`advance_state_unsafe`] instead.
    pub fn advance_state(self, m: Move) -> Option<Self> {
        // First, check that Move major and minor indexes are in range 0..9.
        if m.major > 8 || m.minor > 8 {
            return None;
        }
        // Check that cell is open.
        let sub_board = self.board[m.major as usize];
        let mask = 1 << m.minor;
        if sub_board.x.0 & mask != 0 || sub_board.o.0 & mask != 0 {
            return None;
        }
        // Check that the sub-board is the one the player is supposed to move in.
        if self.next_sub_board != 9 && self.next_sub_board != m.major as u32 {
            return None;
        }
        // Check that the sub-board has not already been won.
        let mask = 1 << m.major;
        if self.sub_wins.x.0 & mask != 0 || self.sub_wins.o.0 & mask != 0 {
            return None;
        }
        // Move is valid, advance the state.
        Some(unsafe { self.advance_state_unsafe(m) })
    }

    pub fn generate_moves_in_place<'a>(&self, moves: &'a mut [Move; 81]) -> &'a [Move] {
        let mut moves_ptr = moves.as_mut_ptr();
        match self.next_sub_board {
            0..=8 => {
                // Can only move in a specific sub-board.
                let sub_board = self.board[self.next_sub_board as usize];
                let or = sub_board.x.0 | sub_board.o.0;
                for i in 0..=8 {
                    if or & 1 << i == 0 {
                        // SAFETY:
                        // This code path will be executed at most 9 times which is below
                        // the buffer size of 81.
                        // Initially, moves_ptr is pointing to the first element of the buffer.
                        // Therefore the first iteration of the loop will write to the first element
                        // of the buffer. Subsequent iterations will write to the next element and
                        // so forth but will never exceed the length of 81.
                        unsafe {
                            *moves_ptr = Move {
                                major: self.next_sub_board,
                                minor: i,
                            };
                            moves_ptr = moves_ptr.add(1);
                        }
                    }
                }
            }
            9 => {
                // Can move in any open spot that is not already won.
                for i in 0..=8 {
                    if self.sub_wins.x.0 & 1 << i == 0
                        && self.sub_wins.o.0 & 1 << i == 0
                        && self.sub_wins.tie.0 & 1 << i == 0
                    {
                        let sub_board = self.board[i];
                        let or = sub_board.x.0 | sub_board.o.0;
                        // Sub-board is available. Generate moves for sub-board.
                        for j in 0..=8 {
                            if or & 1 << j == 0 {
                                // SAFETY:
                                // This code path will be executed at most 81 times which is equal
                                // the buffer size of 81.
                                // Initially, moves_ptr is pointing to the first element of the
                                // buffer. Therefore the first
                                // iteration of the loop will write to the first element
                                // of the buffer. Subsequent iterations will write to the next
                                // element and so forth but will
                                // never exceed the length of 81.
                                unsafe {
                                    *moves_ptr = Move {
                                        major: i as u32,
                                        minor: j,
                                    };
                                    moves_ptr = moves_ptr.add(1);
                                }
                            }
                        }
                    }
                }
            }
            _ => unreachable!("invalid value for self.next_sub_board"),
        }

        // SAFETY: moves_ptr is pointing to an element of buf or address after the last element.
        // It is derived from moves.as_ptr().
        let len = unsafe { moves_ptr.offset_from(moves.as_ptr()) } as usize;
        unsafe { std::slice::from_raw_parts(moves.as_ptr(), len) }
    }

    pub fn generate_moves(&self) -> Vec<Move> {
        let mut buf = [Move::new(0, 0); 81];
        let moves = self.generate_moves_in_place(&mut buf);
        moves.iter().copied().collect()
    }

    pub fn winner(&self) -> Winner {
        if self.sub_wins.x.has_winner() == HasWinner::Yes {
            Winner::X
        } else if self.sub_wins.o.has_winner() == HasWinner::Yes {
            Winner::O
        } else if self.sub_wins.x.0 | self.sub_wins.o.0 | self.sub_wins.tie.0 == 0b111111111 {
            Winner::Tie
        } else {
            Winner::InProgress
        }
    }
}

impl Display for Board {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for major_row in 0..3 {
            for minor_row in 0..3 {
                for major_col in 0..3 {
                    for minor_col in 0..3 {
                        let major = major_row * 3 + major_col;
                        let minor = minor_row * 3 + minor_col;

                        let sub_board = self.board[major];
                        let mask = 1 << minor;
                        if sub_board.x.0 & mask != 0 {
                            write!(f, "X")?;
                        } else if sub_board.o.0 & mask != 0 {
                            write!(f, "O")?;
                        } else {
                            write!(f, "_")?;
                        }

                        write!(f, " ")?;
                    }
                    write!(f, "  ")?;
                }
                writeln!(f)?;
            }
            writeln!(f)?;
        }

        Ok(())
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub struct SubBoard {
    pub x: BitBoard,
    pub o: BitBoard,
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub struct WinBoard {
    pub x: BitBoard,
    pub o: BitBoard,
    pub tie: BitBoard,
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

    /// Returns the bit board with the position of the move applied onto it. Does not change the
    /// original bit board.
    ///
    /// Internally, this function sets the bit corresponding to the position which should be in the
    /// range from `0` to `8` inclusive.
    #[must_use = "advanced_state does not modify original BitBoard"]
    #[inline(always)]
    pub fn advance_bitfield_state(self, pos: u32) -> Self {
        let bit = 1 << pos;
        Self(self.0 | bit)
    }
}

impl BitAnd for BitBoard {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitOr for BitBoard {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

/// Represents a position on the board. Does not store the player who applies the move.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Move {
    /// The major index (position of the sub-board) of the move.
    /// Range can be assumed to be between 0 and 8 inclusive.
    pub major: u32,
    /// The minor index (position of the cell within a sub-board) of the move.
    /// Range can be assumed to be between 0 and 8 inclusive.
    pub minor: u32,
}

impl Move {
    /// Create a new [`Move`].
    ///
    /// # Panics
    /// This method panics if the major index is greater than 8 or the minor index is greater than
    /// 8.
    pub fn new(major: u32, minor: u32) -> Self {
        assert!(major <= 8);
        assert!(minor <= 8);
        Self { major, minor }
    }
}
