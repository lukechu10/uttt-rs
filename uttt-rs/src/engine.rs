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

    /// Expand the node. Returns the expanded node.
    ///
    /// # Panics
    /// This method panics if the node is already fully expanded.
    pub fn expand(&'a self, bump: &'a Bump) -> &'a Self {
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
        next_node_ref
    }

    /// Choose random moves starting from this state until a terminal state is reached.
    ///
    /// The returned [`Winner`] will never be [`Winner::InProgress`].
    /// Also returns the number of moves simulated until the terminal state was reached.
    pub fn rollout(&self) -> (Winner, u32) {
        let mut rng = thread_rng();
        let mut board = self.board;
        let mut moves_count = 0;
        let mut buf = [Move::new(0, 0); 81];
        while board.winner() == Winner::InProgress {
            let moves = board.generate_moves_in_place(&mut buf);
            let m = moves.choose(&mut rng).unwrap();
            // SAFETY: m is a valid Move.
            board = unsafe { board.advance_state_unsafe(*m) };
            moves_count += 1;
        }

        (board.winner(), moves_count)
    }

    pub fn back_propagate(&self, winner: Winner) {
        // Walk up the node tree and increment parent visit/win count.
        let mut next = Some(self);
        while let Some(node) = next {
            if node.board.player_to_move == Player::X && winner == Winner::O
                || node.board.player_to_move == Player::O && winner == Winner::X
            {
                node.wins.set(node.wins.get() + 1.0);
            } else if winner == Winner::Tie {
                node.wins.set(node.wins.get() + 0.5);
            }
            node.visits.set(node.visits.get() + 1);
            next = node.parent;
        }
    }

    pub fn select_best_child_uct(&self) -> Option<&'a Self> {
        let children = self.children.borrow();
        let mut best_child = None;
        let mut best_score = f32::MIN;
        for child in &children.expanded {
            let w = child.wins.get();
            let v = child.visits.get();
            // UCB1 formula.
            let score = (w / v as f32)
                + std::f32::consts::SQRT_2 * f32::sqrt(f32::ln(self.wins.get()) / v as f32);
            if score > best_score {
                best_child = Some(*child);
                best_score = score;
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

    /// Runs MCTS search. Returns the number of iterations performed and moves simulated.
    pub fn run_search(&'a self, time_budget_ms: u128) -> (u32, u32) {
        let start = Instant::now();

        let mut iters = 0;
        let mut moves = 0;
        while start.elapsed().as_millis() < time_budget_ms {
            // Phase 1: selection
            let node = self.root.get().expect("must have a root node").traverse();
            if node.is_fully_expanded() {
                let (winner, moves_count) = node.rollout();
                moves += moves_count;
                node.back_propagate(winner);
                continue;
            }
            // Phase 2: expansion
            let expanded = node.expand(&self.bump);
            // Phase 3: rollout
            let (winner, moves_count) = expanded.rollout();
            moves += moves_count;
            // Phase 4: back-propagation
            expanded.back_propagate(winner);

            iters += 1
        }
        (iters, moves)
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
