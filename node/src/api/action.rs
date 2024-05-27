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
use aze_lib::client::{
    create_aze_client, AzeAccountTemplate, AzeClient, AzeGameMethods, AzeTransactionTemplate,
    PlayCallTransactionData, PlayCheckTransactionData, PlayFoldTransactionData,
    PlayRaiseTransactionData,
};
use aze_lib::constants::BUY_IN_AMOUNT;
use aze_lib::executor::execute_tx_and_sync;
use aze_lib::notes::{consume_notes, mint_note};
use aze_lib::storage::GameStorageSlotData;
use aze_lib::utils::{initial_setup, log_account_status, log_slots};
use aze_types::actions::{GameActionError, GameActionResponse};
use derive_more::Display;
use miden_client::client::{
    accounts::{AccountStorageMode, AccountTemplate},
    transactions::transaction_request::TransactionTemplate,
};
use miden_objects::{
    assets::{Asset, FungibleAsset, TokenSymbol},
    notes::NoteType,
};

#[post("/v1/game/action")]
pub async fn aze_poker_game_action() -> Result<Json<GameActionResponse>, GameActionError> {
    let mut client: AzeClient = create_aze_client();

    let (fungible_asset, sender_account_id, target_account_id, slot_data) =
        initial_setup(&mut client).await;

    let player_bet = slot_data.buy_in_amt(); 

    let playraise_txn_data = PlayRaiseTransactionData::new(
        Asset::Fungible(fungible_asset),
        sender_account_id,
        target_account_id,
        player_bet,
    );
    let transaction_template = AzeTransactionTemplate::PlayRaise(playraise_txn_data);
    let txn_request = client
        .build_aze_play_raise_tx_request(transaction_template)
        .unwrap();
    execute_tx_and_sync(&mut client, txn_request.clone()).await;

    let note_id = txn_request.expected_output_notes()[0].id();
    let note = client.get_input_note(note_id).unwrap();

    let tx_template = TransactionTemplate::ConsumeNotes(target_account_id, vec![note.id()]);
    let tx_request = client.build_transaction_request(tx_template).unwrap();
    execute_tx_and_sync(&mut client, tx_request).await;

    println!("Executed and synced with node");
    log_slots(&client, target_account_id).await;

    Ok(Json(GameActionResponse { is_taken: true }))
}

#[post("/v1/game/call")]
pub async fn aze_poker_game_call() -> Result<Json<GameActionResponse>, GameActionError> {
    let mut client: AzeClient = create_aze_client();

    let (fungible_asset, sender_account_id, target_account_id, slot_data) =
    initial_setup(&mut client).await;
    

    let playcall_txn_data = PlayCallTransactionData::new(
        Asset::Fungible(fungible_asset),
        sender_account_id,
        target_account_id,
    );
    let transaction_template = AzeTransactionTemplate::PlayCall(playcall_txn_data);
    let txn_request = client
        .build_aze_play_call_tx_request(transaction_template)
        .unwrap();
    execute_tx_and_sync(&mut client, txn_request.clone()).await;

    let note_id = txn_request.expected_output_notes()[0].id();
    let note = client.get_input_note(note_id).unwrap();

    let tx_template = TransactionTemplate::ConsumeNotes(target_account_id, vec![note.id()]);
    let tx_request = client.build_transaction_request(tx_template).unwrap();
    execute_tx_and_sync(&mut client, tx_request).await;

    println!("Executed and synced with node");
    log_slots(&client, target_account_id).await;

    Ok(Json(GameActionResponse { is_taken: true }))
}

#[post("/v1/game/fold")]
pub async fn aze_poker_game_fold() -> Result<Json<GameActionResponse>, GameActionError> {
    let mut client: AzeClient = create_aze_client();

    let (fungible_asset, sender_account_id, target_account_id, slot_data) =
    initial_setup(&mut client).await;
    let playcall_txn_data = PlayFoldTransactionData::new(
        Asset::Fungible(fungible_asset),
        sender_account_id,
        target_account_id,
    );
    let transaction_template = AzeTransactionTemplate::PlayFold(playcall_txn_data);
    let txn_request = client
        .build_aze_play_fold_tx_request(transaction_template)
        .unwrap();
    execute_tx_and_sync(&mut client, txn_request.clone()).await;

    let note_id = txn_request.expected_output_notes()[0].id();
    let note = client.get_input_note(note_id).unwrap();

    let tx_template = TransactionTemplate::ConsumeNotes(target_account_id, vec![note.id()]);
    let tx_request = client.build_transaction_request(tx_template).unwrap();
    execute_tx_and_sync(&mut client, tx_request).await;

    println!("Executed and synced with node");
    log_slots(&client, target_account_id).await;

    Ok(Json(GameActionResponse { is_taken: true }))
}

#[post("/v1/game/check")]
pub async fn aze_poker_game_check() -> Result<Json<GameActionResponse>, GameActionError> {
    let mut client: AzeClient = create_aze_client();

    let (fungible_asset, sender_account_id, target_account_id, slot_data) =
    initial_setup(&mut client).await;

    let playcheck_txn_data = PlayCheckTransactionData::new(
        Asset::Fungible(fungible_asset),
        sender_account_id,
        target_account_id,
    );
    let transaction_template = AzeTransactionTemplate::PlayCheck(playcheck_txn_data);
    let txn_request = client
        .build_aze_play_check_tx_request(transaction_template)
        .unwrap();
    execute_tx_and_sync(&mut client, txn_request.clone()).await;

    let note_id = txn_request.expected_output_notes()[0].id();
    let note = client.get_input_note(note_id).unwrap();

    let tx_template = TransactionTemplate::ConsumeNotes(target_account_id, vec![note.id()]);
    let tx_request = client.build_transaction_request(tx_template).unwrap();
    execute_tx_and_sync(&mut client, tx_request).await;

    println!("Executed and synced with node");
    log_slots(&client, target_account_id).await;

    Ok(Json(GameActionResponse { is_taken: true }))
}
