use rand::prelude::SliceRandom;
use rand::thread_rng;
use uttt_rs::*;

fn main() {
    for _i in 0..10 {
        let mut board = Board::new();
        let mut moves = board.generate_moves();
        let mut winner = Winner::InProgress;

        let mut move_counts = Vec::new();

        let mut rng = thread_rng();

        while !moves.is_empty() && winner == Winner::InProgress {
            match board.player_to_move {
                Player::X => {
                    let mcts = MctsEngine::new();
                    mcts.initialize(board);
                    let (_iters, move_count) = mcts.run_search(50);
                    move_counts.push(move_count);
                    let m = mcts.best_move();
                    board = board.advance_state(m).unwrap();
                    moves = board.generate_moves();
                    winner = board.winner();
                }
                Player::O => {
                    let m = *moves.choose(&mut rng).expect("moves is not empty");
                    board = board.advance_state(m).unwrap();
                    moves = board.generate_moves();
                    winner = board.winner();
                }
            }
        }
        let avg_move_count = move_counts.iter().sum::<u32>() / move_counts.len() as u32;
        println!("Winner: {:?}\tAvg. move count: {}", board.winner(), avg_move_count);
    }
}
