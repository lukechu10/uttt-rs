use gloo_timers::future::TimeoutFuture;
use sycamore::{futures::spawn_local_scoped, prelude::*};
use uttt_rs::{Board, MctsEngine, Move, Player, Winner};

#[component]
fn App() -> View {
    view! {
        div(class="container mx-auto font-mono") {
            h1(class="text-xl text-center mb-4 underline") { "Ultimate TicTacToe" }
            GameView {}
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

#[component]
fn GameView() -> View {
    let board = create_signal(Board::new());

    let difficulty = create_signal(100);

    let msg = create_signal("".to_string());
    let move_list = create_signal(Vec::<(Player, Move, Board)>::new());

    // When board changes and player is O, run AI.
    create_effect(move || {
        if board.get().player_to_move == Player::O {
            // Make sure that game is not finished.
            if board.get().winner() != Winner::InProgress {
                return;
            }
            msg.set("Running AI...".to_string());
            // We run the AI in the next micro-task to allow for transitions to finish.
            spawn_local_scoped(async move {
                // Wait 300ms because that is the duration for the transition for sub-board state.
                TimeoutFuture::new(300).await;
                let mcts = MctsEngine::new();
                mcts.initialize(board.get());
                let (iters, moves) = mcts.run_search(difficulty.get_untracked());
                let m = mcts.best_move();
                board.set(board.get().advance_state(m).unwrap());
                msg.set(format!(
                    "AI simulated {} games and {} moves in {}ms.",
                    iters,
                    moves,
                    difficulty.get_untracked()
                ));
                move_list.update(|list| list.push((Player::O, m, board.get())));
            });
        }
    });

    provide_context(move_list);
    provide_context(board);
    view! {
        DifficultySelector(difficulty=difficulty)
        p(class="h-12 py-2") {
            (msg)
        }
        div(class="flex flex-wrap flex-row") {
            GameBoard {}
            MoveHistory {}
        }
    }
}

#[component]
fn GameBoard() -> View {
    view! {
        div(class="game-board mx-auto") {
            ({
                let mut tmp = Vec::new();
                for i in 0..3 {
                    for j in 0..3 {
                        tmp.push(view! { SubBoard(major=(i, j)) })
                    }
                }
                tmp
            })
        }
    }
}

#[component(inline_props)]
fn SubBoard(major: (u32, u32)) -> View {
    let board = use_context::<Signal<Board>>();
    let state = use_sub_board_state(*board, major);
    let class = create_memo(move || match state.get() {
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
                        tmp.push(view! { BoardCell(board=board, major=major, minor=(i, j)) })
                    }
                }
                tmp
            })
        }
    }
}

#[component(inline_props)]
fn BoardCell(board: Signal<Board>, major: (u32, u32), minor: (u32, u32)) -> View {
    let move_list = use_context::<Signal<Vec<(Player, Move, Board)>>>();

    let state = use_board_cell(*board, major, minor);
    let class = create_memo(move || match state.get() {
        Some(Player::X) => "cell x",
        Some(Player::O) => "cell o",
        None => "cell empty",
    });

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
            // Make sure that move is valid. If invalid, do nothing.
            board.set(next);
            move_list.update(|list| list.push((Player::X, m, next)));
        }
    };

    view! {
        div(class=class.get(), on:click=on_click) {
            (match state.get() {
                Some(Player::X) => "X",
                Some(Player::O) => "O",
                None => ""
            })
        }
    }
}

#[component(inline_props)]
fn DifficultySelector(difficulty: Signal<u128>) -> View {
    provide_context(difficulty);
    view! {
        h2(class="text-lg") { "Difficulty:" }
        div(class="flex flex-row space-x-4") {
            Indexed(
                list=create_signal( vec![
                    ("Noob", 50),
                    ("Easy", 100),
                    ("Medium", 500),
                    ("Hard", 1000),
                    ("Boss", 2000),
                    ("Insane", 5000),
                ]),
                view=|(name, value)| view! {
                    DifficultyOption(name=name, value=value)
                },
            )
        }
    }
}

#[component(inline_props)]
fn DifficultyOption(name: &'static str, value: u128) -> View {
    let difficulty = use_context::<Signal<u128>>();
    let class = create_memo(move || {
        if difficulty.get() == value {
            "font-bold underline"
        } else {
            ""
        }
    });
    view! {
        button(class=class.get(), on:click=move |_| difficulty.set(value)) { (name) ": " (value) "ms" }
    }
}

#[component]
fn MoveHistory() -> View {
    let move_list = use_context::<Signal<Vec<(Player, Move, Board)>>>();

    view! {
        div(class="move-history") {
            h2(class="text-lg text-center") { "Moves" }
            table(class="table-auto min-w-[250px] text-center") {
                thead {
                    tr {
                        th(class="w-[80px]") { "Player" }
                        th(class="w-[170px]") { "Move" }
                    }
                }
                tbody {
                    Indexed(
                        list=move_list,
                        view=|(player, m, _)| view! {
                            tr {
                                td { (format!("{:?}", player)) }
                                // Extract row and column from index
                                td {
                                    "(" (m.major / 3 + 1)
                                    ", " (m.major % 3 + 1)
                                    ") (" (m.minor / 3 + 1)
                                    ", " (m.minor % 3 + 1) ")"
                                }
                            }
                        }
                    )
                }
            }
        }
    }
}

fn main() {
    console_error_panic_hook::set_once();

    sycamore::render(App)
}
