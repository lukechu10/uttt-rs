use gloo_timers::future::TimeoutFuture;
use sycamore::{futures::spawn_local_scoped, prelude::*};
use uttt_rs::{Board, MctsEngine, Move, Player, Winner};

#[component]
fn App<G: Html>(cx: Scope) -> View<G> {
    view! { cx,
        div(class="container mx-auto font-mono") {
            h1(class="text-xl text-center mb-4 underline") { "Ultimate TicTacToe" }
            GameView {}
        }
    }
}

fn use_board_cell<'a>(
    cx: Scope<'a>,
    board: &'a ReadSignal<Board>,
    major: (u32, u32),
    minor: (u32, u32),
) -> &'a ReadSignal<Option<Player>> {
    let major_i = major.0 * 3 + major.1;
    let minor_i = minor.0 * 3 + minor.1;

    create_selector(cx, move || {
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

fn use_sub_board_state<'a>(
    cx: Scope<'a>,
    board: &'a ReadSignal<Board>,
    major: (u32, u32),
) -> &'a ReadSignal<SubBoardState> {
    let i = major.0 * 3 + major.1;

    create_selector(cx, move || {
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
fn GameView<G: Html>(cx: Scope) -> View<G> {
    let board = create_signal(cx, Board::new());

    let difficulty = create_signal(cx, 100);

    let msg = create_signal(cx, "".to_string());
    let move_list = create_signal(cx, Vec::<(Player, Move, Board)>::new());

    // When board changes and player is O, run AI.
    create_effect(cx, move || {
        if board.get().player_to_move == Player::O {
            // Make sure that game is not finished.
            if board.get().winner() != Winner::InProgress {
                return;
            }
            msg.set("Running AI...".to_string());
            // We run the AI in the next micro-task to allow for transitions to finish.
            spawn_local_scoped(cx, async move {
                // Wait 300ms because that is the duration for the transition for sub-board state.
                TimeoutFuture::new(300).await;
                let mcts = MctsEngine::new();
                mcts.initialize(*board.get());
                let (iters, moves) = mcts.run_search(*difficulty.get_untracked());
                let m = mcts.best_move();
                board.set(board.get().advance_state(m).unwrap());
                msg.set(format!(
                    "AI simulated {} games and {} moves in {}ms.",
                    iters,
                    moves,
                    *difficulty.get_untracked()
                ));
                move_list.modify().push((Player::O, m, *board.get()));
            });
        }
    });

    provide_context_ref(cx, move_list);
    provide_context_ref(cx, board);
    view! { cx,
        DifficultySelector(difficulty)
        p(class="h-12 py-2") {
            (msg.get())
        }
        div(class="flex flex-wrap flex-row") {
            GameBoard {}
            MoveHistory {}
        }
    }
}

#[component]
fn GameBoard<G: Html>(cx: Scope) -> View<G> {
    view! { cx,
        div(class="game-board mx-auto") {
            ({
                let mut tmp = Vec::new();
                for i in 0..3 {
                    for j in 0..3 {
                        tmp.push(view! { cx, SubBoard((i, j)) })
                    }
                }
                View::new_fragment(tmp)
            })
        }
    }
}

#[component]
fn SubBoard<'a, G: Html>(cx: Scope<'a>, major: (u32, u32)) -> View<G> {
    let board = use_context::<Signal<Board>>(cx);
    let state = use_sub_board_state(cx, board, major);
    let class = create_memo(cx, || match *state.get() {
        SubBoardState::Winner(Winner::X) => "sub-board x",
        SubBoardState::Winner(Winner::O) => "sub-board o",
        SubBoardState::Winner(Winner::Tie) => "sub-board tie",
        SubBoardState::Next => "sub-board next",
        SubBoardState::Winner(Winner::InProgress) => "sub-board in-progress",
    });

    view! { cx,
        div(class=class.get()) {
            ({
                let mut tmp = Vec::new();
                for i in 0..3 {
                    for j in 0..3 {
                        tmp.push(view! { cx, BoardCell((board, major, (i, j))) })
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
#[component]
fn BoardCell<'a, G: Html>(
    cx: Scope<'a>,
    (board, major, minor): (&'a Signal<Board>, (u32, u32), (u32, u32)),
) -> View<G> {
    let move_list = use_context::<Signal<Vec<(Player, Move, Board)>>>(cx);

    let state = use_board_cell(cx, board, major, minor);
    let class = create_memo(cx, || match *state.get() {
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
            move_list.modify().push((Player::X, m, next));
        }
    };

    view! { cx,
        div(class=class.get(), on:click=on_click) {
            (match *state.get() {
                Some(Player::X) => "X",
                Some(Player::O) => "O",
                None => ""
            })
        }
    }
}

#[component]
fn DifficultySelector<'a, G: Html>(cx: Scope<'a>, difficulty: &'a Signal<u128>) -> View<G> {
    provide_context_ref(cx, difficulty);
    view! { cx,
        h2(class="text-lg") { "Difficulty:" }
        div(class="flex flex-row space-x-4") {
            Indexed(
                iterable=create_signal(cx, vec![
                    ("Noob", 50),
                    ("Easy", 100),
                    ("Medium", 500),
                    ("Hard", 1000),
                    ("Boss", 2000),
                    ("Insane", 5000),
                ]),
                view=|cx, (name, value)| view! { cx,
                    DifficultyOption((name, value))
                },
            )
        }
    }
}

#[component]
fn DifficultyOption<'a, G: Html>(cx: Scope<'a>, (name, value): (&'static str, u128)) -> View<G> {
    let difficulty = use_context::<Signal<u128>>(cx);
    let class = create_memo(cx, move || {
        if *difficulty.get() == value {
            "font-bold underline"
        } else {
            ""
        }
    });
    view! { cx,
        button(class=class.get(), on:click=move |_| difficulty.set(value)) { (name) ": " (value) "ms" }
    }
}

#[component]
fn MoveHistory<G: Html>(cx: Scope) -> View<G> {
    let move_list = use_context::<Signal<Vec<(Player, Move, Board)>>>(cx);

    view! { cx,
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
                        iterable=move_list,
                        view=|cx, (player, m, _)| view! { cx,
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
    console_log::init().expect("could not initialize console_log");

    sycamore::render(|cx| {
        view! { cx,
            App {}
        }
    })
}
