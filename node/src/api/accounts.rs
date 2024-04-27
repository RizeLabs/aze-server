use aze_lib::accounts::{create_basic_aze_game_account, create_basic_aze_player_account};
use aze_lib::client::{
    self, create_aze_client, AzeAccountTemplate, AzeClient, AzeGameMethods, AzeTransactionTemplate,
    SendCardTransactionData,
};
use aze_lib::constants::BUY_IN_AMOUNT;
use aze_lib::notes::{consume_notes, mint_note};
use aze_lib::executor::execute_tx_and_sync;
use aze_types::accounts::{AccountCreationError, AccountCreationResponse, PlayerAccountCreationResponse};
use aze_lib::notes::create_send_card_note;
use miden_lib::{transaction, AuthScheme};
use miden_objects::{
    assets::TokenSymbol,
    accounts::{Account, AccountId, AccountStorage, StorageSlotType},
    assembly::ProgramAst,
    assets::{Asset, AssetVault, FungibleAsset},
    crypto::dsa::rpo_falcon512::{PublicKey, SecretKey},
    transaction::TransactionArgs,
    Felt, Word, ONE, ZERO,
    notes::{
        Note, NoteAssets, NoteExecutionMode, NoteId, NoteInputs, NoteMetadata, NoteRecipient,
        NoteScript, NoteTag, NoteType,
    },
    transaction::InputNote,
};
use miden_client::{
    client::{
        accounts::{AccountTemplate, AccountStorageMode},
        transactions::transaction_request::{
            PaymentTransactionData, TransactionRequest, TransactionTemplate,
        },
    },
    store::NoteFilter,
};


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

// TODO: pass account id of the players as request object in this game
#[get("/v1/game/create-account")]
pub async fn create_aze_game_account() -> Result<Json<AccountCreationResponse>, AccountCreationError>
{
    let mut client: AzeClient = create_aze_client();

    // TODO: creating player just for testing purposes 
    let (player_account, _) = client.new_game_account(AzeAccountTemplate::PlayerAccount {
        mutable_code: false,
        storage_mode: AccountStorageMode::Local, // for now
    }).unwrap();

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
    let player_account_ids = vec![player_account.id()];

    let (game_account, _) = client
        .new_game_account(AzeAccountTemplate::GameAccount {
            mutable_code: false,
            storage_mode: AccountStorageMode::Local, // for now
        })
        .unwrap();

    let game_account_id = game_account.id();
    let game_account_storage = game_account.storage();
    println!("Account created: {:?}", game_account_id);
        
    println!("First client consuming note");
    let note =
        mint_note(&mut client, game_account_id, faucet_account_id, NoteType::Public).await;
    println!("Minted note");
        consume_notes(&mut client, game_account_id, &[note]).await;
    println!("Player account consumed note");

    let sender_account_id = game_account_id;

    // let sample_card = [Felt::new(99), Felt::new(99), Felt::new(99), Felt::new(99)];
    let mut cards = vec![];

    for i in 1..2 * player_account_ids.len() + 1 {
        let slot_index = i;
        let card = game_account_storage.get_item(slot_index as u8);
        println!("Card from game storage {:?}", card);
        cards.push(card.into());
    }

    println!("Start sending cards to players");
    for (i, _) in player_account_ids.iter().enumerate() {
        let target_account_id = player_account_ids[i];
        println!("Target account id {:?}", target_account_id);

        let input_cards = [cards[i], cards[i + 1]];
        let sendcard_txn_data = SendCardTransactionData::new(
            Asset::Fungible(fungible_asset),
            sender_account_id,
            target_account_id,
            &input_cards,
        );
        let transaction_template = AzeTransactionTemplate::SendCard(sendcard_txn_data);

        let txn_request = client
            .build_aze_send_card_tx_request(transaction_template)
            .unwrap();

        execute_tx_and_sync(&mut client, txn_request.clone()).await;

        // now we need to consume notes here 
        // get the committed notes
        // let notes = client.get_input_notes(NoteFilter::Committed).unwrap();
        // // TODO: add a check here that notes should not be empty
        // let tx_template = TransactionTemplate::ConsumeNotes(target_account_id, vec![notes[0].id()]);
        let note_id = txn_request.expected_output_notes()[0].id();
        let note = client.get_input_note(note_id).unwrap();

        let tx_template = TransactionTemplate::ConsumeNotes(target_account_id, vec![note.id()]);
        let tx_request = client.build_transaction_request(tx_template).unwrap();
        execute_tx_and_sync(&mut client, tx_request).await;

        println!("Executed and synced with node");

    }

    // create a sample account and compare the storage root for both
    // let (sample_account, _) = client
    //     .new_account(AccountTemplate::FungibleFaucet {
    //         token_symbol: TokenSymbol::new("MATIC").unwrap(),
    //         decimals: 8,
    //         max_supply: 1_000_000_000,
    //         storage_mode: AccountStorageMode::Local,
    //     })
    //     .unwrap();



    // check the store of player 1 account to see are the cards set properly
    let player_account_storage = player_account.storage();
    println!("Player account storage {:?}", player_account_storage.get_item(100));
    println!("Player account storage {:?}", player_account_storage.get_item(101));

    // TODO: define appropriate response types 
    Ok(Json(AccountCreationResponse { is_created: true }))
}

#[get("/v1/player/create-account")]
pub async fn create_aze_player_account(
) -> Result<Json<PlayerAccountCreationResponse>, AccountCreationError> {
    use miden_objects::accounts::AccountType;
    // TODO: get some randomness here to pass it in SecretKey::with_rng method
    let key_pair = SecretKey::new();
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

    Ok(Json(PlayerAccountCreationResponse {
        is_created: true,
        account_id: game_account.id().into(),
    }))
}
