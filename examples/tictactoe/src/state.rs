use crate::tx::{Transaction, TransactionType};
use anyhow::anyhow;
use anyhow::Result;
use prism_common::keys::VerifyingKey;
use std::collections::HashMap;
use std::fmt::Display;
use std::fmt::Formatter;

#[derive(Clone, Hash)]
pub struct Board {
    pub creator: VerifyingKey,
    pub player: Option<VerifyingKey>,

    pub state: [u8; 9],
    pub turn: u8,
}

const WINNING_COMBINATIONS: [[usize; 3]; 8] = [
    [0, 1, 2], // Top row
    [3, 4, 5], // Middle row
    [6, 7, 8], // Bottom row
    [0, 3, 6], // Left column
    [1, 4, 7], // Middle column
    [2, 5, 8], // Right column
    [0, 4, 8], // Diagonal
    [2, 4, 6], // Diagonal
];

impl Board {
    pub fn winner(&self) -> Option<VerifyingKey> {
        for combination in WINNING_COMBINATIONS.iter() {
            let a = self.state[combination[0]];
            let b = self.state[combination[1]];
            let c = self.state[combination[2]];

            if a != 0 && a == b && b == c {
                if a == 1 {
                    return Some(self.creator.clone());
                } else {
                    return self.player.clone();
                }
            }
        }
        None
    }

    pub fn is_full(&self) -> bool {
        self.state.iter().all(|&x| x != 0)
    }

    pub fn is_joined(&self) -> bool {
        self.player.is_some()
    }

    pub fn next_player(&self) -> Option<VerifyingKey> {
        if self.turn % 2 == 0 {
            Some(self.creator.clone())
        } else {
            self.player.clone()
        }
    }
}

impl Display for Board {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        // Convert numbers to symbols for better readability
        let symbol = |n: u8| match n {
            0 => " ",
            1 => "X",
            2 => "O",
            _ => "?",
        };

        // Create the board display with grid lines
        write!(
            f,
            "\n {} | {} | {} \n---+---+---\n {} | {} | {} \n---+---+---\n {} | {} | {} \n",
            symbol(self.state[0]),
            symbol(self.state[1]),
            symbol(self.state[2]),
            symbol(self.state[3]),
            symbol(self.state[4]),
            symbol(self.state[5]),
            symbol(self.state[6]),
            symbol(self.state[7]),
            symbol(self.state[8])
        )
    }
}

#[derive(Default)]
pub struct State {
    pub games: HashMap<String, Board>,
}

impl State {
    pub fn new() -> Self {
        State {
            games: HashMap::new(),
        }
    }
    /// Validates a transaction against the current chain state.
    /// Called during [`process_tx`], but can also be used independently, for
    /// example when queuing transactions to be batched.
    pub(crate) fn validate_tx(&self, tx: Transaction) -> Result<()> {
        tx.verify()?;
        match tx.tx_type {
            TransactionType::Move { game_id, position } => {
                if !self.games.contains_key(&game_id) {
                    return Err(anyhow!("game does not exist"));
                }

                let board = self.games.get(&game_id).unwrap();
                if !board.is_joined() {
                    return Err(anyhow!("this game has not been joined yet!"));
                }

                // if even, player is player, if odd, its creator
                let next_player = board.next_player().unwrap();
                if tx.vk != next_player {
                    return Err(anyhow!("it is not your turn!"));
                }

                if board.state[position as usize] != 0 {
                    return Err(anyhow!("position already taken"));
                }

                if board.winner().is_some() {
                    return Err(anyhow!("game has already been won"));
                }

                Ok(())
            }
            TransactionType::JoinGame { game_id } => {
                if !self.games.contains_key(&game_id) {
                    return Err(anyhow!("game does not exist"));
                }

                let board = self.games.get(&game_id).unwrap();
                if board.is_joined() {
                    return Err(anyhow!("game has already been joined by another player"));
                }
                if board.creator == tx.vk {
                    return Err(anyhow!("you cannot join your own game"));
                }
                Ok(())
            }
            TransactionType::CreateGame { game_id } => {
                if self.games.contains_key(&game_id) {
                    return Err(anyhow!("game already exists"));
                }
                Ok(())
            }
        }
    }
    /// Processes a transaction by validating it and updating the state.
    pub(crate) fn process_tx(&mut self, tx: Transaction) -> Result<()> {
        self.validate_tx(tx.clone())?;
        match tx.tx_type {
            TransactionType::Move { game_id, position } => {
                let board = self.games.get_mut(&game_id).unwrap();
                board.state[position as usize] = if board.turn % 2 == 0 { 1 } else { 2 };
                println!("{}: \n{}", game_id, board);
                if board.winner().is_some() {
                    println!("Game has been won!");
                } else if board.is_full() {
                    println!("Game is a draw!");
                }
                board.turn += 1;
                Ok(())
            }
            TransactionType::JoinGame { game_id } => {
                self.games.get_mut(&game_id).unwrap().player = Some(tx.vk);
                Ok(())
            }
            TransactionType::CreateGame { game_id } => {
                self.games.insert(
                    game_id,
                    Board {
                        creator: tx.vk,
                        player: None,
                        state: [0; 9],
                        turn: 0,
                    },
                );
                Ok(())
            }
        }
    }
}
