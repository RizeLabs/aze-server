pub const DEFAULT_AUTH_SCRIPT: &str =
    "
    use.miden::contracts::auth::basic->auth_tx

    begin
        call.auth_tx::auth_tx_rpo_falcon512
    end
";

pub const CLIENT_CONFIG_FILE_NAME: &str = "miden-client.toml";
pub const BUY_IN_AMOUNT: u64 = 1000;
pub const SMALL_BUY_IN_AMOUNT: u8 = 100;
pub const TRANSFER_AMOUNT: u64 = 59;
pub const SMALL_BLIND_AMOUNT: u8 = 5;
pub const PLAYER_INITIAL_BALANCE: u8 = 30;
pub const HIGHEST_BET: u8 = SMALL_BLIND_AMOUNT;
pub const NO_OF_PLAYERS: u8 = 4;
pub const FLOP_INDEX: u8 = NO_OF_PLAYERS * 2 + 1;
pub const PLAYER_BET_OFFSET: u8 = 3;
pub const IS_FOLD_OFFSET: u8 = 10;
pub const PLAYER_STATS_SLOTS: u8 = 13;
pub const FIRST_PLAYER_INDEX: u8 = 64;
pub const LAST_PLAYER_INDEX: u8 = FIRST_PLAYER_INDEX + (NO_OF_PLAYERS - 1) * PLAYER_STATS_SLOTS;
pub const RAISER_INDEX_SLOT: u8 = 58;
pub const CURRENT_TURN_INDEX_SLOT: u8 = 60;
pub const HIGHEST_BET_SLOT: u8 = 61;
pub const CURRENT_PHASE_SLOT: u8 = 62;
pub const CHECK_COUNTER_SLOT: u8 = 63;
pub const PLAYER_BALANCE_SLOT: u8 = 68;
pub const PLAYER_CARD1_SLOT: u8 = 100;
pub const PLAYER_CARD2_SLOT: u8 = 101;