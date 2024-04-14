use aze_lib::accounts::{create_basic_aze_game_account, create_basic_aze_player_account, get_account_with_custom_account_code};
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

#[post("/v1/game/deal")]
pub async fn deal() ->  Result<Json<DealingResponse>, DealingError> {
    // - input  
    // - fetch all the player id, participating in a particular game using account_get_item
    // for now hardcoding player ids

     // Create an asset
     let faucet_id = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
     let fungible_asset: Asset = FungibleAsset::new(faucet_id, 100).unwrap().into();
 
     // Create sender and target account
     let sender_account_id = AccountId::try_from(ACCOUNT_ID_SENDER).unwrap();
 
     let target_account_id =
         AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN).unwrap();
     let (target_pub_key, target_sk_pk_felt) = get_new_key_pair_with_advice_map();
     let target_account =
     get_account_with_custom_account_code(target_account_id, target_pub_key, None);
 
     // Create the note
    //  let note = create_deal_note(
    //      sender_account_id,
    //      target_account_id,
    //      vec![fungible_asset],
    //      RpoRandomCoin::new([Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)]),
    //  )
    //  .unwrap();
 
     // CONSTRUCT AND EXECUTE TX (Success)
     // --------------------------------------------------------------------------------------------
    //  let data_store =
    //      MockDataStore::with_existing(Some(target_account.clone()), Some(vec![note.clone()]));
 
    //  let mut executor = TransactionExecutor::new(data_store.clone());
    //  executor.load_account(target_account_id).unwrap();
 
    //  let block_ref = data_store.block_header.block_num();
    //  let note_ids = data_store.notes.iter().map(|note| note.id()).collect::<Vec<_>>();
 
    //  let tx_script_code = ProgramAst::parse(DEFAULT_AUTH_SCRIPT).unwrap();
 
    //  let tx_script_target = executor
    //      .compile_tx_script(
    //          tx_script_code.clone(),
    //          vec![(target_pub_key, target_sk_pk_felt)],
    //          vec![],
    //      )
    //      .unwrap();
    //  let tx_args_target = TransactionArgs::new(Some(tx_script_target), None);
 
    //  // Execute the transaction and get the witness
    //  let executed_transaction = executor
    //      .execute_transaction(target_account_id, block_ref, &note_ids, Some(tx_args_target))
    //      .unwrap();
 
     // Prove, serialize/deserialize and verify the transaction
     // We can add this as a last step
     //assert!(prove_and_verify_transaction(executed_transaction.clone()).is_ok());
 
     // Not sure what you want to test after the account but we should see if the 
     // account change is what you expect
    //  let mut target_storage = target_account.storage().clone();
    //  target_storage.set_item(100, [Felt::new(99), Felt::new(99), Felt::new(99), Felt::new(99)]).unwrap();
    //  target_storage.set_item(101, [Felt::new(98), Felt::new(98), Felt::new(98), Felt::new(98)]).unwrap();
     
    //  let target_account_after: Account = Account::new(
    //      target_account.id(),
    //      AssetVault::new(&[fungible_asset]).unwrap(),
    //      target_storage,
    //      target_account.code().clone(),
    //      Felt::new(2),
    //  );

    Ok(Json(DealingResponse { is_dealt: true }))    
}