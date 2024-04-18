pub const DEFAULT_AUTH_SCRIPT: &str = "
    use.miden::contracts::auth::basic->auth_tx

    begin
        call.auth_tx::auth_tx_rpo_falcon512
    end
";

pub const CLIENT_CONFIG_FILE_NAME: &str = "miden-client.toml";
pub const BUY_IN_AMOUNT: u64 = 1000;
pub const TRANSFER_AMOUNT: u64 = 59;