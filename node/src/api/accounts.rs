use aze_lib::accounts::create_basic_aze_player_account;
use aze_lib::client::{
    self,
    create_aze_client,
    AzeAccountTemplate,
    AzeClient,
    AzeGameMethods,
    AzeTransactionTemplate,
    SendCardTransactionData,
};
use aze_lib::constants::BUY_IN_AMOUNT;
use aze_lib::notes::{ consume_notes, mint_note };
use aze_lib::executor::execute_tx_and_sync;
use aze_lib::storage::GameStorageSlotData;

use aze_types::accounts::{
    AccountCreationError,
    AccountCreationRequest,
    AccountCreationResponse,
    PlayerAccountCreationRequest,
    PlayerAccountCreationResponse,
};
use aze_lib::utils::log_account_status;
use miden_lib::AuthScheme;
use miden_objects::{
    accounts:: AccountId,
    assets::TokenSymbol,
    assets::{ Asset, FungibleAsset },
    crypto::dsa::rpo_falcon512::{ PublicKey, SecretKey },
    notes::NoteType,
};
use miden_client::client::{
    accounts::{ AccountTemplate, AccountStorageMode },
    transactions::transaction_request::TransactionTemplate,
};

use actix_web::{ get, post, web::Json };

// TODO: pass account id of the players as request object in this game
#[post("/v1/game/create-account")]
pub async fn create_aze_game_account(request_object: Json<AccountCreationRequest>) -> Result<
    Json<AccountCreationResponse>,
    AccountCreationError
> {
    let mut client: AzeClient = create_aze_client();
    let slot_data = GameStorageSlotData::new(0, 0, 0, 0, 0, 0);

    let (faucet_account, _) = client
        .new_account(AccountTemplate::FungibleFaucet {
            token_symbol: TokenSymbol::new("MATIC").unwrap(),
            decimals: 8,
            max_supply: 1_000_000_000,
            storage_mode: AccountStorageMode::Local,
        })
        .unwrap();

    let faucet_account_id = faucet_account.id();
    let fungible_asset = FungibleAsset::new(faucet_account_id, BUY_IN_AMOUNT).unwrap();

    // TODO: get the player account ids from the request object
    let player_account_ids = request_object.game_player_ids.clone();

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
    let game_account_storage = game_account.storage();

    println!("Account created: {:?}", game_account_id);

    println!("First client consuming note");
    let note = mint_note(&mut client, game_account_id, faucet_account_id, NoteType::Public).await;
    println!("Minted note");
    consume_notes(&mut client, game_account_id, &[note]).await;
    println!("Player account consumed note");

    let sender_account_id = game_account_id;

    let mut cards = vec![];

    for i in 1..2 * player_account_ids.len() + 1 {
        let slot_index = i;
        let card = game_account_storage.get_item(slot_index as u8);
        println!("Card from game storage {:?}", card);
        cards.push(card.into());
    }

    println!("Start sending cards to players");
    for (i, _) in player_account_ids.iter().enumerate() {
        let target_account_id = AccountId::try_from(player_account_ids[i]).unwrap();
        println!("Target account id {:?}", target_account_id);

        let input_cards = [cards[i], cards[i + 1]];
        let sendcard_txn_data = SendCardTransactionData::new(
            Asset::Fungible(fungible_asset),
            sender_account_id,
            target_account_id,
            &input_cards
        );
        let transaction_template = AzeTransactionTemplate::SendCard(sendcard_txn_data);

        let txn_request = client.build_aze_send_card_tx_request(transaction_template).unwrap();

        execute_tx_and_sync(&mut client, txn_request.clone()).await;

        let note_id = txn_request.expected_output_notes()[0].id();
        let note = client.get_input_note(note_id).unwrap();

        let tx_template = TransactionTemplate::ConsumeNotes(target_account_id, vec![note.id()]);
        let tx_request = client.build_transaction_request(tx_template).unwrap();
        execute_tx_and_sync(&mut client, tx_request).await;

        println!("Executed and synced with node");
    }

    // TODO: define appropriate response types
    Ok(Json(AccountCreationResponse { game_id: game_account_id.into() }))
}

#[post("/v1/player/create-account")]
pub async fn create_aze_player_account(request_object: Json<PlayerAccountCreationRequest>) -> Result<
    Json<PlayerAccountCreationResponse>,
    AccountCreationError
> {
    use miden_objects::accounts::AccountType;
    // TODO: get some randomness here to pass it in SecretKey::with_rng method
    let key_pair = SecretKey::new();
    let pub_key: PublicKey = key_pair.public_key();
    let auth_scheme: AuthScheme = AuthScheme::RpoFalcon512 { pub_key };

    // we need to use an initial seed to create the wallet account
    let init_seed: [u8; 32] = [
        95, 113, 209, 94, 84, 105, 250, 242, 223, 203, 216, 124, 22, 159, 14, 132, 215, 85, 183, 204,
        149, 90, 166, 68, 100, 73, 106, 168, 125, 237, 138, 16,
    ];

    let (game_account, _) = create_basic_aze_player_account(
        init_seed,
        auth_scheme,
        AccountType::RegularAccountImmutableCode
    ).unwrap();

    Ok(
        Json(PlayerAccountCreationResponse {
            account_id: game_account.id().into(),
        })
    )
}
