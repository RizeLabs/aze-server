use aze_lib::accounts::{create_basic_aze_game_account, create_basic_aze_player_account};
use aze_lib::client::{self, create_aze_client, AzeAccountTemplate, AzeClient, AzeGameMethods};
use miden_lib::AuthScheme;
use miden_objects::{
    accounts::{Account, AccountId, AccountStorage, StorageSlotType},
    assembly::ProgramAst,
    assets::{Asset, AssetVault, FungibleAsset},
    crypto::dsa::rpo_falcon512::{KeyPair, PublicKey},
    transaction::TransactionArgs,
    Felt, Word, ONE, ZERO,
};
use miden_tx::TransactionExecutor;

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
use miden_client::client::accounts::AccountStorageMode;

#[derive(Deserialize, Serialize)]
pub struct AccountCreationResponse {
    is_created: bool,
}

#[derive(Debug, Display)]
pub enum AccountCreationError {
    AccountCreationFailed,
    BadTaskRequest,
}

impl ResponseError for AccountCreationError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::json())
            .body(self.to_string())
    }

    fn status_code(&self) -> StatusCode {
        match self {
            AccountCreationError::AccountCreationFailed => StatusCode::FAILED_DEPENDENCY,
            AccountCreationError::BadTaskRequest => StatusCode::BAD_REQUEST,
        }
    }
}

#[get("/v1/game/create-account")]
pub async fn create_aze_game_account() -> Result<Json<AccountCreationResponse>, AccountCreationError>
{
    let mut client: AzeClient = create_aze_client();

    let (game_account, _) = client
        .new_game_account(AzeAccountTemplate::GameAccount {
            mutable_code: false,
            storage_mode: AccountStorageMode::Local, // for now
        })
        .unwrap();
    let game_account_id = game_account.id();
    println!("Account created: {:?}", game_account_id);

    // println!("Account by this client {:?} ", client.get_accounts());
    // let val = game_account.storage().get_item(1);
    // println!("Account storage value: {:?}", val);

    // println!("Account created: {:?}", game_account);

    Ok(Json(AccountCreationResponse { is_created: true }))
}

#[get("/v1/player/create-account")]
pub async fn create_aze_player_account(
) -> Result<Json<AccountCreationResponse>, AccountCreationError> {
    use miden_objects::accounts::AccountType;
    let key_pair: KeyPair = KeyPair::new().unwrap();
    let pub_key: PublicKey = key_pair.public_key();
    let auth_scheme: AuthScheme = AuthScheme::RpoFalcon512 { pub_key };

    // we need to use an initial seed to create the wallet account
    let init_seed: [u8; 32] = [
        95, 113, 209, 94, 84, 105, 250, 242, 223, 203, 216, 124, 22, 159, 14, 132, 215, 85, 183,
        204, 149, 90, 166, 68, 100, 73, 106, 168, 125, 237, 138, 16,
    ];

    let (game_account, _) = create_basic_aze_player_account(
        init_seed,
        auth_scheme,
        AccountType::RegularAccountImmutableCode,
    )
    .unwrap();
    // println!("Account created: {:?}", game_account);

    Ok(Json(AccountCreationResponse { is_created: true }))
}
