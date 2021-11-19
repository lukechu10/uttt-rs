//! MCTS algorithm.

use std::cell::{Cell, RefCell};

use bumpalo::Bump;
use instant::Instant;
use rand::prelude::SliceRandom;
use rand::thread_rng;

use crate::{Board, Move, Player, Winner};

#[derive(Clone)]
struct NodeChildren<'a> {
    expanded: Vec<&'a Node<'a>>,
    unexpanded: Vec<Move>,
}

/// Node in MCTS.
#[derive(Clone)]
pub struct Node<'a> {
    parent: Option<&'a Self>,
    children: RefCell<NodeChildren<'a>>,
    board: Board,
    is_terminal: bool,
    previous_move: Option<Move>,

    wins: Cell<f32>,
    visits: Cell<u32>,
}

impl<'a> Node<'a> {
    pub fn new(parent: Option<&'a Self>, board: Board, previous_move: Option<Move>) -> Self {
        let mut unexpanded = board.generate_moves();

        // Shuffle unexpanded nodes.
        // TODO: do not create a thread_rng each time.
        let mut rng = thread_rng();
        unexpanded.shuffle(&mut rng);

        let children = NodeChildren {
            expanded: Vec::new(),
            unexpanded,
        };

        let is_terminal = board.winner() != Winner::InProgress;

        Self {
            parent,
            children: RefCell::new(children),
            board,
            is_terminal,
            previous_move,
            wins: Cell::new(0.0),
            visits: Cell::new(0),
        }
    }

    pub fn is_fully_expanded(&self) -> bool {
        self.children.borrow().unexpanded.is_empty()
    }

    /// Expand the node.
    ///
    /// # Panics
    /// This method panics if the node is already fully expanded.
    pub fn expand(&'a self, bump: &'a Bump) {
        let m = self
            .children
            .borrow_mut()
            .unexpanded
            .pop()
            .expect("node cannot be fully expanded");

        // Expand node.
        // SAFETY: m is a valid Move.
        let next = unsafe { self.board.advance_state_unsafe(m) };
        let next_node = Node::new(Some(self), next, Some(m));
        let next_node_ref = bump.alloc(next_node);
        self.children.borrow_mut().expanded.push(next_node_ref);
    }

    /// Choose random moves starting from this state until a terminal state is reached.
    ///
    /// The returned [`Winner`] will never be [`Winner::InProgress`].
    pub fn rollout(&self) -> Winner {
        let mut rng = thread_rng();
        let mut board = self.board;
        while board.winner() == Winner::InProgress {
            let moves = board.generate_moves();
            let m = moves.choose(&mut rng).unwrap();
            // SAFETY: m is a valid Move.
            board = unsafe { board.advance_state_unsafe(*m) };
        }

        board.winner()
    }

    pub fn back_propagate(&self, winner: Winner) {
        // Increment self visit count.
        self.visits.set(self.visits.get() + 1);
        // Increment self win count.
        if self.board.player_to_move == Player::X && winner == Winner::O
            || self.board.player_to_move == Player::O && winner == Winner::X
        {
            self.wins.set(self.wins.get() + 1.0);
        } else if winner == Winner::Tie {
            self.wins.set(self.wins.get() + 0.5);
        }
        // Walk up the node tree and increment parent visit/win count.
        let mut node = self;
        while let Some(parent) = node.parent {
            if parent.board.player_to_move == Player::X && winner == Winner::O
                || parent.board.player_to_move == Player::O && winner == Winner::X
            {
                self.wins.set(self.wins.get() + 1.0);
            } else if winner == Winner::Tie {
                self.wins.set(self.wins.get() + 0.5);
            }
            parent.visits.set(parent.visits.get() + 1);
            node = parent;
        }
    }

    pub fn uct(&self) -> f32 {
        let n = self.visits.get();
        let w = self.wins.get();
        let c = 1.0;
        let n_plus_c = n as f32 + c;
        w / n_plus_c + c * (2.0 * std::f32::consts::SQRT_2) / n_plus_c
    }

    pub fn select_best_child_uct(&self) -> Option<&'a Self> {
        let mut best_child = None;
        let mut best_uct = 0.0;
        let children = self.children.borrow();
        for child in children.expanded.iter() {
            let uct = child.uct();
            if uct > best_uct {
                best_child = Some(*child);
                best_uct = uct;
            }
        }
        best_child
    }

    /// # Panics
    /// This method panics if the engine is not initialized. Initialize the engine with
    /// `initialize()` first.
    pub fn traverse(&'a self) -> &'a Self {
        // Start at the root node.
        let mut node = self;
        while node.is_fully_expanded() && !node.is_terminal {
            match node.select_best_child_uct() {
                Some(tmp) => node = tmp,
                None => break,
            }
        }

        node
    }
}

pub struct MctsEngine<'a> {
    bump: Bump,
    root: Cell<Option<&'a Node<'a>>>,
}

impl<'a> MctsEngine<'a> {
    pub fn new() -> Self {
        let bump = Bump::new();

        Self {
            bump,
            root: Cell::new(None),
        }
    }

    pub fn initialize(&'a self, board: Board) {
        let root = self.bump.alloc(Node::new(None, board, None));
        self.root.set(Some(root));
    }

    /// Runs MCTS search. Returns the number of iterations performed.
    pub fn run_search(&'a self, time_budget_ms: u128) -> u32 {
        let start = Instant::now();

        let mut i = 0;
        while start.elapsed().as_millis() < time_budget_ms {
            // Phase 1: selection
            let node = self.root.get().expect("must have a root node").traverse();
            if node.is_fully_expanded() {
                let winner = node.rollout();
                node.back_propagate(winner);
                continue;
            }
            // Phase 2: expansion
            node.expand(&self.bump);
            // Phase 3: rollout
            let winner = node.rollout();
            // Phase 4: back-propagation
            node.back_propagate(winner);

            i+= 1
        }
        i
    }

    /// # Panics
    /// Panics if the engine is not initialized. Panics if no moves available for the given state.
    pub fn best_move(&self) -> Move {
        let node = self.root.get().expect("must have a root node");

        // Find best child node.
        let children = node.children.borrow();
        children
            .expanded
            .iter()
            .max_by_key(|x| x.visits.get())
            .expect("state does not have any valid moves")
            .previous_move
            .unwrap()
    }
}

impl<'a> Default for MctsEngine<'a> {
    fn default() -> Self {
        Self::new()
    }
}
