use sycamore::prelude::*;
use uttt_rs::{Board, Move, Player};

#[component(App<G>)]
fn app() -> View<G> {
    view! {
        div(class="container mx-auto font-mono") {
            h1(class="text-xl") { "Ultimate TicTacToe" }
            GameView()
        }
    }
}

fn use_board_cell(
    board: ReadSignal<Board>,
    major: (u32, u32),
    minor: (u32, u32),
) -> ReadSignal<Option<Player>> {
    let major_i = major.0 * 3 + major.1;
    let minor_i = minor.0 * 3 + minor.1;

    create_selector(move || {
        let sub_board = board.get().board[major_i as usize];
        let mask = 1 << minor_i;
        if sub_board.x.0 & mask != 0 {
            Some(Player::X)
        } else if sub_board.o.0 & mask != 0 {
            Some(Player::O)
        } else {
            None
        }
    })
}

#[component(GameView<G>)]
fn game_view() -> View<G> {
    let board = Signal::new(Board::new());

    view! {
        GameBoard(board)
    }
}

#[component(GameBoard<G>)]
fn game_board(board: Signal<Board>) -> View<G> {
    view! {
        div(class="game-board mx-auto") {
            ({
                let mut tmp = Vec::new();
                for i in 0..3 {
                    for j in 0..3 {
                        tmp.push(view! { SubBoard((board.clone(), (i, j))) })
                    }
                }
                View::new_fragment(tmp)
            })
        }
    }
}

/// # Props
/// - `0`: `Signal<Board>`, the game board state.
/// - `1`: `(u32, u32)`, the position of the sub-board.
#[component(SubBoard<G>)]
fn sub_board(props: (Signal<Board>, (u32, u32))) -> View<G> {
    let (board, major) = props;
    view! {
        div(class="sub-board") {
            ({
                let mut tmp = Vec::new();
                for i in 0..3 {
                    for j in 0..3 {
                        tmp.push(view! { BoardCell((board.clone(), major, (i, j))) })
                    }
                }
                View::new_fragment(tmp)
            })
        }
    }
}

/// # Props
/// - `0`: `Signal<Board>`, the game board state.
/// - `1`: `(u32, u32)`, the position of the sub-board.
/// - `2`: `(u32, u32)`, the position of the cell within the sub-board.
#[component(BoardCell<G>)]
fn board_cell(props: (Signal<Board>, (u32, u32), (u32, u32))) -> View<G> {
    let (board, major, minor) = props;
    let state = use_board_cell(board.handle(), major, minor);
    let class = create_memo(cloned!(state => move || {
        match *state.get() {
            Some(Player::X) => "cell x",
            Some(Player::O) => "cell o",
            None => "cell",
        }
    }));

    let on_click = move |_| {
        // Update board.
        let m = Move::new(major.0 * 3 + major.1, minor.0 * 3 + minor.1);
        // SAFETY: m is valid because it is constructed with Move::new.
        unsafe {
            board.set(board.get().advance_state(m));
        }
    };

    view! {
        div(class=class.get(), on:click=on_click) {
            (match *state.get() {
                Some(Player::X) => "X",
                Some(Player::O) => "O",
                None => ""
            })
        }
    }
}

fn main() {
    sycamore::render(|| {
        view! {
            App()
        }
    })
}
