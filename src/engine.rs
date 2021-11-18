//! MCTS algorithm.

use std::cell::{Cell, RefCell};

use bumpalo::Bump;

use crate::{Board, Move};

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

    wins: Cell<f32>,
    visits: Cell<u32>,
}

impl<'a> Node<'a> {
    // pub fn new(parent: Option<&'a Self>, board: Board) -> Self {
    //     let unexpanded_moves = board.generate_moves();

    //     Self {
    //         parent,
    //         wins: Cell::new(0.0),
    //         visits: Cell::new(0),
    //     }
    // }
}

pub struct MctsEngine<'a> {
    bump: Bump,
    root: Option<&'a Node<'a>>,
}

impl<'a> MctsEngine<'a> {
    pub fn new() -> Self {
        let bump = Bump::new();

        Self { bump, root: None }
    }

    pub fn initialize(&'a mut self, board: Board) {
        let root = self.bump.alloc(Node {
            parent: None,
            children: RefCell::new(NodeChildren {
                expanded: Vec::new(),
                unexpanded: Vec::new(),
            }),
            board,
            wins: Cell::new(0.0),
            visits: Cell::new(0),
        });
        self.root = Some(root);
    }
}

impl<'a> Default for MctsEngine<'a> {
    fn default() -> Self {
        Self::new()
    }
}
