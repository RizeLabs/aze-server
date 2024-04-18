use aze_lib::accounts::{create_basic_aze_game_account, create_basic_aze_player_account};
use aze_lib::client::create_aze_client;
use aze_lib::notes::create_send_card_note;
use aze_lib::utils::{get_new_key_pair_with_advice_map};
use aze_lib::constants::DEFAULT_AUTH_SCRIPT;
use crate::model::accounts::Task;
use crate::model::accounts::TaskState;
use miden_lib::AuthScheme;
use miden_objects::{
    accounts::{Account, AccountId, AccountStorage, StorageSlotType},
    assembly::ProgramAst,
    assets::{Asset, AssetVault, FungibleAsset},
    crypto::dsa::rpo_falcon512::{SecretKey, PublicKey},
    crypto::rand::RpoRandomCoin,
    transaction::TransactionArgs,
    Felt, Word, ONE, ZERO,
};
use miden_tx::TransactionExecutor;
// use miden_mock::mock::account::{
//     ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN,
//     ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN, ACCOUNT_ID_SENDER, DEFAULT_AUTH_SCRIPT,
// };

use actix_web::{
    error::ResponseError,
    get,
    http::{header::ContentType, StatusCode},
    post, put,
    web::Data,
    web::Json,
    web::Path,
    HttpResponse,
};
use derive_more::Display;
use serde::{Deserialize, Serialize};

pub const ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN: u64 = 3238098370154045919;
pub const ACCOUNT_ID_SENDER: u64 = 0b0110111011u64 << 54;
pub const ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN: u64 = 0b1010111100 << 54;
pub const ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN: u64 = 0b1110011100 << 54;
pub const ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN_1: u64 = 0b1110011101 << 54;
pub const FUNGIBLE_ASSET_AMOUNT: u64 = 100;
pub const FUNGIBLE_FAUCET_INITIAL_BALANCE: u64 = 50000;

pub const ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1: u64 =
    0b1010010001111111010110100011011110101011010001101111110110111100u64;
pub const ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2: u64 =
    0b1010000101101010101101000110111101010110100011011110100011011101u64;
pub const ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_3: u64 =
    0b1010011001011010101101000110111101010110100011011101000110111100u64;

#[derive(Deserialize, Serialize)]
pub struct DealingResponse {
    is_dealt: bool,
}

#[derive(Debug, Display)]
pub enum DealingError {
    DealingFailed,
    BadRequest
}

impl ResponseError for DealingError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::json())
            .body(self.to_string())
    }

    fn status_code(&self) -> StatusCode {
        match self {
            DealingError::DealingFailed => StatusCode::FAILED_DEPENDENCY,
            DealingError::BadRequest => StatusCode::BAD_REQUEST
        }
    }
}
