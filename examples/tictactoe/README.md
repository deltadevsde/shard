# TicTacToe Example

## Created with
```bash
shard init tictactoe && cd tictactoe
shard create-tx CreateGame game_id String
shard create-tx JoinGame game_id String
shard create-tx Move game_id String position u8
```

First, activate signature verification in `state.rs` by setting the const to true.
Then modified `src/state.rs` and `src/tx.rs` to write the transaction processing rules and state management.
1. You can create a game by passing a non-used `game_id`
2. You can join the game with a JoinGame tx from a different signer
3. The creator then has the first move. The creator submits a move transaction to progress game state
4. Once the game is won, no more moves can be made.

## Run with

First, create your signers
```bash
tictactoe create-signer player1
tictactoe create-signer player2
```

Then,
```bash
tictactoe serve

tictactoe submit-tx --key-name player1 create-game test_game
tictactoe submit-tx --key-name player2 join-game test_game

tictactoe submit-tx --key-name player1 move test_game 0
tictactoe submit-tx --key-name player2 move test_game 4
tictactoe submit-tx --key-name player1 move test_game 3
```

## Improvement ideas:
1. Nonce control is not implemented, replay attacks can occur
2. Improve state management and provide state reads in `state.rs` and offer them over a web interface to play online
	- list all games
		- for a given user
		- all unjoined games
	- get user stats
	- register usernames to not use verifying keys directly
	- create a random game_id instead of passing a new one in

