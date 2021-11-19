use gloo_timers::future::TimeoutFuture;
use sycamore::futures::spawn_local_in_scope;
use sycamore::prelude::*;
use uttt_rs::{Board, MctsEngine, Move, Player, Winner};

#[component(App<G>)]
fn app() -> View<G> {
    view! {
        div(class="container mx-auto font-mono") {
            h1(class="text-xl text-center mb-4 underline") { "Ultimate TicTacToe" }
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum SubBoardState {
    Winner(Winner),
    Next,
}

fn use_sub_board_state(board: ReadSignal<Board>, major: (u32, u32)) -> ReadSignal<SubBoardState> {
    let i = major.0 * 3 + major.1;

    create_selector(move || {
        // Check win state of sub-board.
        let sub_board = board.get().sub_wins;
        let mask = 1 << i;
        if sub_board.x.0 & mask != 0 {
            SubBoardState::Winner(Winner::X)
        } else if sub_board.o.0 & mask != 0 {
            SubBoardState::Winner(Winner::O)
        } else if sub_board.tie.0 & mask != 0 {
            SubBoardState::Winner(Winner::Tie)
        } else if board.get().next_sub_board == 9 || board.get().next_sub_board == i {
            SubBoardState::Next
        } else {
            SubBoardState::Winner(Winner::InProgress)
        }
    })
}

#[component(GameView<G>)]
fn game_view() -> View<G> {
    let board = Signal::new(Board::new());
    let msg = Signal::new("".to_string());

    // When board changes and player is O, run AI.
    create_effect(cloned!(board, msg => move || {
        const TIME_BUDGET_MS: u128 = 500;
        if board.get().player_to_move == Player::O {
            // Make sure that game is not finished.
            if board.get().winner() != Winner::InProgress {
                return;
            }
            msg.set("Running AI...".to_string());
            // We run the AI in the next micro-task to allow for transitions to finish.
            spawn_local_in_scope(cloned!(board, msg => async move {
                // Wait 300ms because that is the duration for the transition for sub-board state.
                TimeoutFuture::new(300).await;
                let mcts = MctsEngine::new();
                mcts.initialize(*board.get());
                let (iters, moves) = mcts.run_search(TIME_BUDGET_MS);
                let m = mcts.best_move();
                board.set(board.get().advance_state(m).unwrap());

                msg.set(format!("AI simulated {} games and {} moves in {}ms.", iters, moves, TIME_BUDGET_MS));
            }));
        }
    }));

    view! {
        p(class="h-12 py-2") {
            (msg.get())
        }
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
    let state = use_sub_board_state(board.handle(), major);
    let class = create_memo(move || match *state.get() {
        SubBoardState::Winner(Winner::X) => "sub-board x",
        SubBoardState::Winner(Winner::O) => "sub-board o",
        SubBoardState::Winner(Winner::Tie) => "sub-board tie",
        SubBoardState::Next => "sub-board next",
        SubBoardState::Winner(Winner::InProgress) => "sub-board in-progress",
    });

    view! {
        div(class=class.get()) {
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
            None => "cell empty",
        }
    }));

    let on_click = move |_| {
        // Make sure that it is player X's turn. TODO: allow user to choose player.
        if board.get().player_to_move != Player::X {
            return;
        }
        // Make sure that game is not finished.
        if board.get().winner() != Winner::InProgress {
            return;
        }
        // Update board.
        let m = Move::new(major.0 * 3 + major.1, minor.0 * 3 + minor.1);
        let next = board.get().advance_state(m);
        if let Some(next) = next {
            board.set(next);
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
    console_error_panic_hook::set_once();
    console_log::init().expect("could not initialize console_log");

    sycamore::render(|| {
        view! {
            App()
        }
    })
}
