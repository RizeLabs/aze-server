use aze_lib::client::{
    AzeClient,
    AzeGameMethods,
    AzeAccountTemplate,
    AzeTransactionTemplate,
    SendCardTransactionData,
    PlayRaiseTransactionData,
    PlayCallTransactionData,
};
use aze_lib::constants::BUY_IN_AMOUNT;
use aze_lib::executor::execute_tx_and_sync;
use aze_lib::utils::{ get_random_coin, load_config };
use aze_lib::notes::{ consume_notes, mint_note };
use aze_lib::storage::GameStorageSlotData;
use miden_client::{
    client::{
        accounts::{ AccountTemplate, AccountStorageMode },
        transactions::transaction_request::TransactionTemplate,
        rpc::TonicRpcClient,
    },
    config::{ ClientConfig, RpcConfig },
    errors::{ ClientError, NodeRpcClientError },
    store::sqlite_store::SqliteStore,
};
use miden_crypto::hash::rpo::RpoDigest;
use miden_crypto::FieldElement;
use miden_objects::{
    Felt,
    assets::{ TokenSymbol, FungibleAsset, Asset },
    accounts::AccountId,
    notes::NoteType,
};
use std::{ env::temp_dir, time::Duration };
// use uuid::Uuid;

fn create_test_client() -> AzeClient {
    let mut current_dir = std::env
        ::current_dir()
        .map_err(|err| err.to_string())
        .unwrap();
    current_dir.push("miden-client.toml");
    let client_config = load_config(current_dir.as_path()).unwrap();

    println!("Client Config: {:?}", client_config);

    let rpc_endpoint = client_config.rpc.endpoint.to_string();
    let store = SqliteStore::new((&client_config).into()).unwrap();
    let executor_store = SqliteStore::new((&client_config).into()).unwrap();
    let rng = get_random_coin();
    AzeClient::new(TonicRpcClient::new(&rpc_endpoint), rng, store, executor_store, true)
}

async fn wait_for_node(client: &mut AzeClient) {
    const NODE_TIME_BETWEEN_ATTEMPTS: u64 = 5;
    const NUMBER_OF_NODE_ATTEMPTS: u64 = 60;

    println!(
        "Waiting for Node to be up. Checking every {NODE_TIME_BETWEEN_ATTEMPTS}s for {NUMBER_OF_NODE_ATTEMPTS} tries..."
    );

    for _try_number in 0..NUMBER_OF_NODE_ATTEMPTS {
        match client.sync_state().await {
            Err(ClientError::NodeRpcClientError(NodeRpcClientError::ConnectionError(_))) => {
                std::thread::sleep(Duration::from_secs(NODE_TIME_BETWEEN_ATTEMPTS));
            }
            Err(other_error) => {
                panic!("Unexpected error: {other_error}");
            }
            _ => {
                return;
            }
        }
    }

    panic!("Unable to connect to node");
}

#[tokio::test]
async fn test_create_aze_game_account() {
    let mut client = create_test_client();

    // TODO: remove this constants from here
    let small_blind_amt = 5u8;
    let buy_in_amt = 100u8;
    let no_of_players = 4u8;
    let flop_index = no_of_players * 2 + 1;
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

    // TODO: somehow manage the game seed as well
    let (game_account, _) = client
        .new_game_account(
            AzeAccountTemplate::GameAccount {
                mutable_code: false,
                storage_mode: AccountStorageMode::Local,
            },
            Some(slot_data)
        )
        .unwrap();
    let game_account_storage = game_account.storage();

    let mut slot_index = 1;

    // check are the cards has been correctly placed
    for card_suit in 1..5 {
        for card_number in 1..14 {
            let slot_item = RpoDigest::new([
                Felt::from(card_suit as u8),
                Felt::from(card_number as u8),
                Felt::ZERO, // denotes is encrypted
                Felt::ZERO,
            ]);

            assert_eq!(game_account_storage.get_item(slot_index), slot_item);

            slot_index = slot_index + 1;
        }
    }

    // checking next turn
    assert_eq!(
        game_account_storage.get_item(slot_index),
        RpoDigest::new([Felt::from(flop_index as u8), Felt::ZERO, Felt::ZERO, Felt::ZERO])
    );

    slot_index = slot_index + 1;

    // checking the small blind amount
    assert_eq!(
        game_account_storage.get_item(slot_index),
        RpoDigest::new([Felt::from(small_blind_amt), Felt::ZERO, Felt::ZERO, Felt::ZERO])
    );

    slot_index = slot_index + 1;

    // checking the big blind amount
    assert_eq!(
        game_account_storage.get_item(slot_index),
        RpoDigest::new([Felt::from(small_blind_amt * 2), Felt::ZERO, Felt::ZERO, Felt::ZERO])
    );

    slot_index = slot_index + 1;

    // checking the buy in amount
    assert_eq!(
        game_account_storage.get_item(slot_index),
        RpoDigest::new([Felt::from(buy_in_amt), Felt::ZERO, Felt::ZERO, Felt::ZERO])
    );

    slot_index = slot_index + 1;
    // checking no of player slot
    assert_eq!(
        game_account_storage.get_item(slot_index),
        RpoDigest::new([Felt::from(no_of_players), Felt::ZERO, Felt::ZERO, Felt::ZERO])
    );

    slot_index = slot_index + 1;
    // checking flop index slot
    assert_eq!(
        game_account_storage.get_item(slot_index),
        RpoDigest::new([Felt::ZERO, Felt::ZERO, Felt::ZERO, Felt::ZERO])
    );
}

#[tokio::test]
async fn test_cards_distribution() {
    let mut client: AzeClient = create_test_client();

    let slot_data = GameStorageSlotData::new(0, 0, 0, 0, 0, 0);

    let (game_account, _) = client
        .new_game_account(
            AzeAccountTemplate::GameAccount {
                mutable_code: false,
                storage_mode: AccountStorageMode::Local,
            },
            Some(slot_data)
        )
        .unwrap();

    let game_account_id = game_account.id();
    let game_account_storage = game_account.storage();

    // TODO: for now we''ll distribute cards to two players
    let (player1_account, _) = client
        .new_game_account(
            AzeAccountTemplate::PlayerAccount {
                mutable_code: false,
                storage_mode: AccountStorageMode::Local,
            },
            None
        )
        .unwrap();

    let (player2_account, _) = client
        .new_game_account(
            AzeAccountTemplate::PlayerAccount {
                mutable_code: false,
                storage_mode: AccountStorageMode::Local,
            },
            None
        )
        .unwrap();

    // setting up faucet account here
    let (faucet_account, _) = client
        .new_account(AccountTemplate::FungibleFaucet {
            token_symbol: TokenSymbol::new("MATIC").unwrap(),
            decimals: 8,
            max_supply: 1_000_000_000,
            storage_mode: AccountStorageMode::Local,
        })
        .unwrap();

    let faucet_account_id = faucet_account.id();
    fund_account(&mut client, game_account_id, faucet_account_id).await;

    let fungible_asset = FungibleAsset::new(faucet_account_id, BUY_IN_AMOUNT).unwrap();

    let player_account_ids = vec![player1_account.id(), player2_account.id()];

    let mut cards: Vec<[Felt; 4]> = vec![];

    for slot_index in 1..2 * player_account_ids.len() + 1 {
        let slot_item = game_account_storage.get_item(slot_index as u8);
        cards.push(slot_item.into());
    }

    println!("Card {:?}", cards);

    println!("Start sending cards to players");
    for (i, _) in player_account_ids.iter().enumerate() {
        let target_account_id = player_account_ids[i];
        println!("Target account id {:?}", target_account_id);

        let input_cards = [cards[i], cards[i + 1]]; // don't you think the input cards should contain 8 felt -> 2 cards
        let sendcard_txn_data = SendCardTransactionData::new(
            Asset::Fungible(fungible_asset),
            game_account_id,
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
        assert_account_status(&client, target_account_id, i).await;
    }
}

#[tokio::test]
async fn test_play_raise() {
    let mut client: AzeClient = create_test_client();

    let small_blind_amt = 5u8;
    let buy_in_amt = 100u8;
    let no_of_players = 4u8;
    let flop_index = no_of_players * 2 + 1;
    let current_turn_index = 65u8;
    let player_balance = 10u8;

    let game_slot_data = GameStorageSlotData::new(
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
                storage_mode: AccountStorageMode::Local,
            },
            Some(game_slot_data.clone())
        )
        .unwrap();

    let game_account_storage = game_account.storage();

    let (player_account, _) = client
        .new_game_account(
            AzeAccountTemplate::PlayerAccount {
                mutable_code: false,
                storage_mode: AccountStorageMode::Local,
            },
            None
        )
        .unwrap();

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

    let sender_account_id = player_account.id();
    let target_account_id = game_account.id();

    fund_account(&mut client, sender_account_id, faucet_account_id).await;

    let player_bet = small_blind_amt;

    let playraise_txn_data = PlayRaiseTransactionData::new(
        Asset::Fungible(fungible_asset),
        sender_account_id,
        target_account_id,
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
    assert_slot_status_raise(&client, target_account_id, game_slot_data).await;
}

#[tokio::test]
async fn test_play_call() {
    let mut client: AzeClient = create_test_client();

    let small_blind_amt = 5u8;
    let buy_in_amt = 100u8;
    let no_of_players = 4u8;
    let flop_index = no_of_players * 2 + 1;
    let current_turn_index = 65u8;
    let player_balance = 10u8;
    let highest_bet = small_blind_amt;

    let game_slot_data = GameStorageSlotData::new(
        small_blind_amt,
        buy_in_amt,
        no_of_players,
        current_turn_index,
        highest_bet,
        player_balance
    );

    let (game_account, _) = client
        .new_game_account(
            AzeAccountTemplate::GameAccount {
                mutable_code: false,
                storage_mode: AccountStorageMode::Local,
            },
            Some(game_slot_data.clone())
        )
        .unwrap();

    let game_account_storage = game_account.storage();

    let (player_account, _) = client
        .new_game_account(
            AzeAccountTemplate::PlayerAccount {
                mutable_code: false,
                storage_mode: AccountStorageMode::Local,
            },
            None
        )
        .unwrap();

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

    let sender_account_id = player_account.id();
    let target_account_id = game_account.id();

    fund_account(&mut client, sender_account_id, faucet_account_id).await;

    let playraise_txn_data = PlayCallTransactionData::new(
        Asset::Fungible(fungible_asset),
        sender_account_id,
        target_account_id
    );

    let transaction_template = AzeTransactionTemplate::PlayCall(playraise_txn_data);
    let txn_request = client.build_aze_play_call_tx_request(transaction_template).unwrap();
    execute_tx_and_sync(&mut client, txn_request.clone()).await;

    let note_id = txn_request.expected_output_notes()[0].id();
    let note = client.get_input_note(note_id).unwrap();

    let tx_template = TransactionTemplate::ConsumeNotes(target_account_id, vec![note.id()]);
    let tx_request = client.build_transaction_request(tx_template).unwrap();
    execute_tx_and_sync(&mut client, tx_request).await;

    println!("Executed and synced with node");
    assert_slot_status_call(&client, target_account_id, game_slot_data).await;
}

async fn assert_account_status(client: &AzeClient, account_id: AccountId, index: usize) {
    let (account, _) = client.get_account(account_id).unwrap();
    let card_suit = 1u8;

    assert_eq!(account.vault().assets().count(), 1);
    assert_eq!(
        account.storage().get_item(100),
        RpoDigest::new([
            Felt::from(card_suit),
            Felt::from((index + 1) as u8),
            Felt::ZERO,
            Felt::ZERO,
        ])
    );
    assert_eq!(
        account.storage().get_item(101),
        RpoDigest::new([
            Felt::from(card_suit),
            Felt::from((index + 2) as u8),
            Felt::ZERO,
            Felt::ZERO,
        ])
    );
}

async fn assert_slot_status_raise(
    client: &AzeClient,
    account_id: AccountId,
    slot_data: GameStorageSlotData
) {
    let (account, _) = client.get_account(account_id).unwrap();
    let game_account_storage = account.storage();

    let small_blind_amt = slot_data.small_blind_amt();
    let buy_in_amt = slot_data.buy_in_amt();
    let no_of_players = slot_data.player_count();
    let flop_index = slot_data.flop_index();

    let mut slot_index = 1;

    // check are the cards has been correctly placed
    for card_suit in 1..5 {
        for card_number in 1..14 {
            let slot_item = RpoDigest::new([
                Felt::from(card_suit as u8),
                Felt::from(card_number as u8),
                Felt::ZERO, // denotes is encrypted
                Felt::ZERO,
            ]);

            assert_eq!(game_account_storage.get_item(slot_index), slot_item);

            slot_index = slot_index + 1;
        }
    }

    // checking next turn
    assert_eq!(
        game_account_storage.get_item(slot_index),
        RpoDigest::new([Felt::from(flop_index as u8), Felt::ZERO, Felt::ZERO, Felt::ZERO])
    );

    slot_index = slot_index + 1;

    // checking the small blind amount
    assert_eq!(
        game_account_storage.get_item(slot_index),
        RpoDigest::new([Felt::from(small_blind_amt), Felt::ZERO, Felt::ZERO, Felt::ZERO])
    );

    slot_index = slot_index + 1;

    // checking the big blind amount
    assert_eq!(
        game_account_storage.get_item(slot_index),
        RpoDigest::new([Felt::from(small_blind_amt * 2), Felt::ZERO, Felt::ZERO, Felt::ZERO])
    );

    slot_index = slot_index + 1;

    // checking the buy in amount
    assert_eq!(
        game_account_storage.get_item(slot_index),
        RpoDigest::new([Felt::from(buy_in_amt), Felt::ZERO, Felt::ZERO, Felt::ZERO])
    );

    slot_index = slot_index + 1;
    // checking no of player slot
    assert_eq!(
        game_account_storage.get_item(slot_index),
        RpoDigest::new([Felt::from(no_of_players), Felt::ZERO, Felt::ZERO, Felt::ZERO])
    );

    slot_index = slot_index + 1;
    // checking raiser
    assert_eq!(
        game_account_storage.get_item(slot_index),
        RpoDigest::new([
            Felt::from(slot_data.current_turn_index()),
            Felt::ZERO,
            Felt::ZERO,
            Felt::ZERO,
        ])
    );

    slot_index = slot_index + 2;
    // check current player index
    assert_eq!(
        game_account_storage.get_item(slot_index),
        RpoDigest::new([
            Felt::from(slot_data.current_turn_index()),
            Felt::ZERO,
            Felt::ZERO,
            Felt::ZERO,
        ])
    );

    slot_index = slot_index + 1;
    // check highest bet
    assert_eq!(
        game_account_storage.get_item(slot_index),
        RpoDigest::new([Felt::from(slot_data.highest_bet()), Felt::ZERO, Felt::ZERO, Felt::ZERO])
    );

    slot_index = slot_index + 6;
    let player_bet = small_blind_amt;

    // check player bet
    assert_eq!(
        game_account_storage.get_item(slot_index),
        RpoDigest::new([Felt::from(player_bet), Felt::ZERO, Felt::ZERO, Felt::ZERO])
    );

    let remaining_balance = slot_data.player_balance() - player_bet;

    slot_index = slot_index + 1;
    // check player balance
    assert_eq!(
        game_account_storage.get_item(slot_index),
        RpoDigest::new([Felt::from(remaining_balance), Felt::ZERO, Felt::ZERO, Felt::ZERO])
    );
}

async fn assert_slot_status_call(
    client: &AzeClient,
    account_id: AccountId,
    slot_data: GameStorageSlotData
) {
    let (account, _) = client.get_account(account_id).unwrap();
    let game_account_storage = account.storage();

    let remaining_balance = slot_data.player_balance() - slot_data.highest_bet();
    let slot_index = 68;

    // check player balance
    assert_eq!(
        game_account_storage.get_item(slot_index),
        RpoDigest::new([Felt::from(remaining_balance), Felt::ZERO, Felt::ZERO, Felt::ZERO])
    );
}

async fn fund_account(client: &mut AzeClient, account_id: AccountId, faucet_account_id: AccountId) {
    let note_1 = mint_note(client, account_id, faucet_account_id, NoteType::Public).await;
    consume_notes(client, account_id, &[note_1]).await;
    let note_2 = mint_note(client, account_id, faucet_account_id, NoteType::Public).await;
    consume_notes(client, account_id, &[note_2]).await;
}