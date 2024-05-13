pub const DEFAULT_AUTH_SCRIPT: &str =
    "
    use.miden::contracts::auth::basic->auth_tx

    begin
        call.auth_tx::auth_tx_rpo_falcon512
    end
";

pub const CLIENT_CONFIG_FILE_NAME: &str = "miden-client.toml";
pub const BUY_IN_AMOUNT: u64 = 1000;
pub const TRANSFER_AMOUNT: u64 = 59;
pub const SMALL_BLIND_AMOUNT: u8 = 5;
pub const NO_OF_PLAYERS: u8 = 4;
pub const FLOP_INDEX: u8 = NO_OF_PLAYERS * 2 + 1;
pub const CURRENT_TURN_INDEX: u8 = 65;
pub const PLAYER_INITIAL_BALANCE: u8 = 10;
pub const HIGHEST_BET: u8 = SMALL_BLIND_AMOUNT;
pub const PLAYER_BALANCE_SLOT: u8 = 68;
pub const CURRENT_TURN_INDEX_SLOT: u8 = 60;
pub const PLAYER_STATS_SLOTS: u8 = 13;
pub const CHECK_COUNTER_SLOT: u8 = 63;