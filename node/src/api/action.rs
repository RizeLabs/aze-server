use actix_web::{
    error::ResponseError,
    get,
    http::{ header::ContentType, StatusCode },
    post,
    put,
    web::Data,
    web::Json,
    web::Path,
    HttpResponse,
};
use derive_more::Display;
use aze_types::actions::{ GameActionError, GameActionResponse };
use aze_lib::utils::{ log_account_status, log_slots };
use aze_lib::storage::GameStorageSlotData;
use aze_lib::executor::execute_tx_and_sync;
use aze_lib::constants::BUY_IN_AMOUNT;
use aze_lib::client::{
    AzeClient,
    AzeAccountTemplate,
    create_aze_client,
    PlayRaiseTransactionData,
    AzeTransactionTemplate,
    AzeGameMethods,
};
use miden_client::client::{
    accounts::{ AccountStorageMode, AccountTemplate },
    transactions::transaction_request::TransactionTemplate,
};
use miden_objects::{ assets::{ TokenSymbol, Asset, FungibleAsset }, notes::NoteType };
use aze_lib::notes::{ consume_notes, mint_note };

#[post("/v1/game/action")]
pub async fn aze_poker_game_action() -> Result<Json<GameActionResponse>, GameActionError> {
    let mut client: AzeClient = create_aze_client();

    let small_blind_amt = 5u8;
    let buy_in_amt = 100u8;
    let no_of_players = 4u8;
    let current_turn_index = 65u8;
    let player_balance = 10u8;

    let slot_data = GameStorageSlotData::new(
        small_blind_amt,
        buy_in_amt,
        no_of_players,
        current_turn_index,
        small_blind_amt,
        player_balance
    );

    let (game_account, _) = client
        .new_game_account(
            AzeAccountTemplate::GameAccount {
                mutable_code: false,
                storage_mode: AccountStorageMode::Local, // for now
            },
            Some(slot_data)
        )
        .unwrap();
    let game_account_id = game_account.id();
    log_slots(&client, game_account_id).await;

    let (player_account, _) = client
        .new_game_account(
            AzeAccountTemplate::PlayerAccount {
                mutable_code: false,
                storage_mode: AccountStorageMode::Local, // for now
            },
            None
        )
        .unwrap();
    let player_account_id = player_account.id();

    let (faucet_account, _) = client
        .new_account(AccountTemplate::FungibleFaucet {
            token_symbol: TokenSymbol::new("MATIC").unwrap(),
            decimals: 8,
            max_supply: 1_000_000_000,
            storage_mode: AccountStorageMode::Local,
        })
        .unwrap();
    let faucet_account_id = faucet_account.id();

    let note = mint_note(&mut client, player_account_id, faucet_account_id, NoteType::Public).await;
    println!("Minted note");
    consume_notes(&mut client, player_account_id, &[note]).await;

    let fungible_asset = FungibleAsset::new(faucet_account_id, BUY_IN_AMOUNT).unwrap();
    let sender_account_id = player_account_id;
    let target_account_id = game_account_id;

    let player_bet = small_blind_amt;

    let playraise_txn_data = PlayRaiseTransactionData::new(
        Asset::Fungible(fungible_asset),
        sender_account_id,
        game_account_id,
        player_bet
    );
    let transaction_template = AzeTransactionTemplate::PlayRaise(playraise_txn_data);
    let txn_request = client.build_aze_play_raise_tx_request(transaction_template).unwrap();
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
