# aze-server

### Build and run 
- Clone the repo
- change the directory `cd aze-server` 
- build the project `cargo build`
- run the server `cargo run --release`

### Run test
- `cargo test --release --test=integration`

### Endpoints
- `/v1/game/create-account`: This endpoint will orchestrate the local store with some player accounts and game account. And will deal the card among from game account to player account
- `/v1/player/create-account`: This endpoint will create player account.
