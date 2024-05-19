use aze_lib::client::{
    AzeClient,
    AzeGameMethods,
    AzeAccountTemplate,
    AzeTransactionTemplate,
    SendCardTransactionData,
    PlayBetTransactionData,
    PlayRaiseTransactionData,
    PlayCallTransactionData,
    PlayFoldTransactionData,
    PlayCheckTransactionData,
};
use aze_lib::constants::{
    BUY_IN_AMOUNT,
    SMALL_BLIND_AMOUNT,
    NO_OF_PLAYERS,
    FLOP_INDEX,
    IS_FOLD_OFFSET,
    FIRST_PLAYER_INDEX,
    LAST_PLAYER_INDEX,
    HIGHEST_BET,
    PLAYER_INITIAL_BALANCE,
    PLAYER_BALANCE_SLOT,
    CURRENT_TURN_INDEX_SLOT,
    CHECK_COUNTER_SLOT,
    RAISER_INDEX_SLOT,
    PLAYER_STATS_SLOTS,
    HIGHEST_BET_SLOT,
    CURRENT_PHASE_SLOT
};
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
    accounts::{ Account, AccountId },
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

fn setup_accounts(
    client: &mut AzeClient
) -> (Account, AccountId, AccountId, GameStorageSlotData) {
    let slot_data = GameStorageSlotData::new(
        SMALL_BLIND_AMOUNT,
        BUY_IN_AMOUNT as u8,
        NO_OF_PLAYERS,
        FIRST_PLAYER_INDEX,
        HIGHEST_BET,
        PLAYER_INITIAL_BALANCE
    );

    let (game_account, _) = client
        .new_game_account(
            AzeAccountTemplate::GameAccount {
                mutable_code: false,
                storage_mode: AccountStorageMode::Local,
            },
            Some(slot_data.clone())
        )
        .unwrap();

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

    return (game_account, player_account.id(), faucet_account.id(), slot_data);
}

#[tokio::test]
async fn test_create_aze_game_account() {
    let mut client = create_test_client();

    // TODO: somehow manage the game seed as well
    let (game_account, _, _, _) = setup_accounts(&mut client);
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
        RpoDigest::new([Felt::from(FLOP_INDEX), Felt::ZERO, Felt::ZERO, Felt::ZERO])
    );

    slot_index = slot_index + 1;

    // checking the small blind amount
    assert_eq!(
        game_account_storage.get_item(slot_index),
        RpoDigest::new([Felt::from(SMALL_BLIND_AMOUNT), Felt::ZERO, Felt::ZERO, Felt::ZERO])
    );

    slot_index = slot_index + 1;

    // checking the big blind amount
    assert_eq!(
        game_account_storage.get_item(slot_index),
        RpoDigest::new([Felt::from(SMALL_BLIND_AMOUNT * 2), Felt::ZERO, Felt::ZERO, Felt::ZERO])
    );

    slot_index = slot_index + 1;

    // checking the buy in amount
    assert_eq!(
        game_account_storage.get_item(slot_index),
        RpoDigest::new([Felt::from(BUY_IN_AMOUNT as u8), Felt::ZERO, Felt::ZERO, Felt::ZERO])
    );

    slot_index = slot_index + 1;
    // checking no of player slot
    assert_eq!(
        game_account_storage.get_item(slot_index),
        RpoDigest::new([Felt::from(NO_OF_PLAYERS), Felt::ZERO, Felt::ZERO, Felt::ZERO])
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

    let (game_account, player1_account_id, faucet_account_id, _) = setup_accounts(&mut client);

    let game_account_id = game_account.id();
    let game_account_storage = game_account.storage();

    let (player2_account, _) = client
        .new_game_account(
            AzeAccountTemplate::PlayerAccount {
                mutable_code: false,
                storage_mode: AccountStorageMode::Local,
            },
            None
        )
        .unwrap();

    fund_account(&mut client, game_account_id, faucet_account_id).await;
    fund_account(&mut client, game_account_id, faucet_account_id).await;

    let fungible_asset = FungibleAsset::new(faucet_account_id, BUY_IN_AMOUNT).unwrap();

    let player_account_ids = vec![player1_account_id, player2_account.id()];

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

    let (game_account, player_account_id, faucet_account_id, game_slot_data) = setup_accounts(
        &mut client
    );

    let game_account_storage = game_account.storage();

    let fungible_asset = FungibleAsset::new(faucet_account_id, BUY_IN_AMOUNT).unwrap();

    let sender_account_id = player_account_id;
    let target_account_id = game_account.id();

    fund_account(&mut client, sender_account_id, faucet_account_id).await;

    let player_bet = SMALL_BLIND_AMOUNT;

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

    let (game_account, player_account_id, faucet_account_id, game_slot_data) = setup_accounts(
        &mut client
    );

    let game_account_storage = game_account.storage();

    let fungible_asset = FungibleAsset::new(faucet_account_id, BUY_IN_AMOUNT).unwrap();

    let sender_account_id = player_account_id;
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

#[tokio::test]
async fn test_play_fold() {
    let mut client: AzeClient = create_test_client();

    let (game_account, player_account_id, faucet_account_id, game_slot_data) = setup_accounts(
        &mut client
    );

    let game_account_storage = game_account.storage();

    let fungible_asset = FungibleAsset::new(faucet_account_id, BUY_IN_AMOUNT).unwrap();

    let sender_account_id = player_account_id;
    let target_account_id = game_account.id();

    fund_account(&mut client, sender_account_id, faucet_account_id).await;

    let playfold_txn_data = PlayFoldTransactionData::new(
        Asset::Fungible(fungible_asset),
        sender_account_id,
        target_account_id
    );

    let transaction_template = AzeTransactionTemplate::PlayFold(playfold_txn_data);
    let txn_request = client.build_aze_play_fold_tx_request(transaction_template).unwrap();
    execute_tx_and_sync(&mut client, txn_request.clone()).await;

    let note_id = txn_request.expected_output_notes()[0].id();
    let note = client.get_input_note(note_id).unwrap();

    let tx_template = TransactionTemplate::ConsumeNotes(target_account_id, vec![note.id()]);
    let tx_request = client.build_transaction_request(tx_template).unwrap();
    execute_tx_and_sync(&mut client, tx_request).await;

    println!("Executed and synced with node");
    assert_slot_status_fold(&client, target_account_id, game_slot_data).await;
}

#[tokio::test]
async fn test_play_check() {
    let mut client: AzeClient = create_test_client();

    let (game_account, player_account_id, faucet_account_id, game_slot_data) = setup_accounts(
        &mut client
    );

    let game_account_storage = game_account.storage();

    let fungible_asset = FungibleAsset::new(faucet_account_id, BUY_IN_AMOUNT).unwrap();
    let sender_account_id = player_account_id;
    let target_account_id = game_account.id();

    fund_account(&mut client, sender_account_id, faucet_account_id).await;

    let playcheck_txn_data = PlayCheckTransactionData::new(
        Asset::Fungible(fungible_asset),
        sender_account_id,
        target_account_id
    );

    let transaction_template = AzeTransactionTemplate::PlayCheck(playcheck_txn_data);
    let txn_request = client.build_aze_play_check_tx_request(transaction_template).unwrap();
    execute_tx_and_sync(&mut client, txn_request.clone()).await;

    let note_id = txn_request.expected_output_notes()[0].id();
    let note = client.get_input_note(note_id).unwrap();

    let tx_template = TransactionTemplate::ConsumeNotes(target_account_id, vec![note.id()]);
    let tx_request = client.build_transaction_request(tx_template).unwrap();
    execute_tx_and_sync(&mut client, tx_request).await;

    println!("Executed and synced with node");
    assert_slot_status_check(&client, target_account_id, game_slot_data.clone(), 1 as u8).await;
}

// #[tokio::test]
async fn test_e2e() {
    let mut client: AzeClient = create_test_client();

    let (game_account, player1_account_id, faucet_account_id, game_slot_data) = setup_accounts(
        &mut client
    );

    let game_account_id = game_account.id();

    let (player2_account, _) = client
        .new_game_account(
            AzeAccountTemplate::PlayerAccount {
                mutable_code: false,
                storage_mode: AccountStorageMode::Local,
            },
            None
        )
        .unwrap();
    
    let (player3_account, _) = client
        .new_game_account(
            AzeAccountTemplate::PlayerAccount {
                mutable_code: false,
                storage_mode: AccountStorageMode::Local,
            },
            None
        )
        .unwrap();

    let (player4_account, _) = client
        .new_game_account(
            AzeAccountTemplate::PlayerAccount {
                mutable_code: false,
                storage_mode: AccountStorageMode::Local,
            },
            None
        )
        .unwrap();

    // Player 1 --> Small blind bets SMALL_BLIND_AMOUNT
    let player1_bet = SMALL_BLIND_AMOUNT;
    bet(&mut client, player1_account_id, game_account_id, faucet_account_id, player1_bet, 1 as u8).await;
    //check player balance
    let game_account = client.get_account(game_account_id).unwrap().0;
    assert_eq!(
        game_account.storage().get_item(68 as u8),
        RpoDigest::new([Felt::from((PLAYER_INITIAL_BALANCE - player1_bet) as u8), Felt::ZERO, Felt::ZERO, Felt::ZERO])
    );
    println!("----->>> Small blind betted");
    log_slots(&client, game_account_id).await;
    // Player 2 --> Big blind bets SMALL_BLIND_AMOUNT * 2
    let player2_bet = SMALL_BLIND_AMOUNT * 2;
    bet(&mut client, player2_account.id(), game_account_id, faucet_account_id, player2_bet, 2 as u8).await;
    // check balance
    let game_account = client.get_account(game_account_id).unwrap().0;
    assert_eq!(
        game_account.storage().get_item(81 as u8),
        RpoDigest::new([Felt::from((PLAYER_INITIAL_BALANCE - player2_bet) as u8), Felt::ZERO, Felt::ZERO, Felt::ZERO])
    );
    println!("----->>> Big blind betted");
    log_slots(&client, game_account_id).await;

    // Deal cards to players and assert the account status
    // deal_card(&mut client, game_account_id, player1_account_id, faucet_account_id, 0).await;
    // deal_card(&mut client, game_account_id, player2_account.id(), faucet_account_id, 2).await;
    // deal_card(&mut client, game_account_id, player3_account.id(), faucet_account_id, 4).await;
    // deal_card(&mut client, game_account_id, player4_account.id(), faucet_account_id, 6).await;
    println!("----->>> Cards distributed");

    // Player 3 --> Call
    call(&mut client, player3_account.id(), game_account_id, faucet_account_id, 3).await;
    // check balance
    let game_account = client.get_account(game_account_id).unwrap().0;
    assert_eq!(
        game_account.storage().get_item(94 as u8),
        RpoDigest::new([Felt::from(20 as u8), Felt::ZERO, Felt::ZERO, Felt::ZERO])
    );
    println!("----->>> Player 3 called");
    log_slots(&client, game_account_id).await;

    // Player 4 --> Fold
    fold(&mut client, player4_account.id(), game_account_id, faucet_account_id, 4).await;
    println!("----->>> Player 4 folded");
    log_slots(&client, game_account_id).await;

    // Player 1 --> Call
    call(&mut client, player1_account_id, game_account_id, faucet_account_id, 1).await;
    // check balance
    let game_account = client.get_account(game_account_id).unwrap().0;
    assert_eq!(
        game_account.storage().get_item(68 as u8),
        RpoDigest::new([Felt::from(20 as u8), Felt::ZERO, Felt::ZERO, Felt::ZERO])
    );
    println!("----->>> Player 1 called");
    log_slots(&client, game_account_id).await;

    // Player 2 --> Check
    println!("----->>> Big blind checking...");
    check(&mut client, player2_account.id(), game_account_id, faucet_account_id, 2).await;
    log_slots(&client, game_account_id).await;
    // assert check counter
    let game_account = client.get_account(game_account_id).unwrap().0;
    assert_eq!(
        game_account.storage().get_item(CURRENT_PHASE_SLOT),
        RpoDigest::new([Felt::from(1 as u8), Felt::ZERO, Felt::ZERO, Felt::ZERO])
    );
    println!("----->>> Player 2 checked");
    println!("----->>> Flop revealed");

    // Player 1 --> Check
    check(&mut client, player1_account_id, game_account_id, faucet_account_id, 1).await;
    println!("----->>> Player 1 checked");
    log_slots(&client, game_account_id).await;
    // Player 2 --> Check
    check(&mut client, player2_account.id(), game_account_id, faucet_account_id, 2).await;
    println!("----->>> Player 2 checked");
    log_slots(&client, game_account_id).await;
    // Player 3 --> Check
    check(&mut client, player3_account.id(), game_account_id, faucet_account_id, 3).await;
    println!("----->>> Player 3 checked");
    log_slots(&client, game_account_id).await;
    println!("----->>> Turn revealed");
    
    let game_account = client.get_account(game_account_id).unwrap().0;
    assert_eq!(
        game_account.storage().get_item(CURRENT_PHASE_SLOT),
        RpoDigest::new([Felt::from(2 as u8), Felt::ZERO, Felt::ZERO, Felt::ZERO])
    );

    // Player 1 --> Check
    check(&mut client, player1_account_id, game_account_id, faucet_account_id, 1).await;
    println!("----->>> Player 1 checked");
    log_slots(&client, game_account_id).await;
    // Player 2 --> Raise
    raise(&mut client, player2_account.id(), game_account_id, faucet_account_id, 3 * SMALL_BLIND_AMOUNT, 2).await;
    // check balance
    let game_account = client.get_account(game_account_id).unwrap().0;
    assert_eq!(
        game_account.storage().get_item(81 as u8),
        RpoDigest::new([Felt::from(5 as u8), Felt::ZERO, Felt::ZERO, Felt::ZERO])
    );
    println!("----->>> Player 2 raised");
    log_slots(&client, game_account_id).await;
    // Player 3 --> Call
    call(&mut client, player3_account.id(), game_account_id, faucet_account_id, 3).await;
    println!("----->>> Player 3 called");
    log_slots(&client, game_account_id).await;
    // Player 1 --> Call
    call(&mut client, player1_account_id, game_account_id, faucet_account_id, 1).await;
    println!("----->>> Player 1 called");
    log_slots(&client, game_account_id).await;
    println!("----->>> River revealed");

    let game_account = client.get_account(game_account_id).unwrap().0;
    assert_eq!(
        game_account.storage().get_item(CURRENT_PHASE_SLOT),
        RpoDigest::new([Felt::from(3 as u8), Felt::ZERO, Felt::ZERO, Felt::ZERO])
    );
    
    // Player 1 --> Check
    check(&mut client, player1_account_id, game_account_id, faucet_account_id, 1).await;
    println!("----->>> Player 1 checked");
    log_slots(&client, game_account_id).await;
    // Player 2 --> Check
    check(&mut client, player2_account.id(), game_account_id, faucet_account_id, 2).await;
    println!("----->>> Player 2 checked");
    log_slots(&client, game_account_id).await;
    // Player 3 --> Check
    check(&mut client, player3_account.id(), game_account_id, faucet_account_id, 3).await;
    println!("----->>> Player 3 checked");
    log_slots(&client, game_account_id).await;

    let game_account = client.get_account(game_account_id).unwrap().0;
    assert_eq!(
        game_account.storage().get_item(CURRENT_PHASE_SLOT),
        RpoDigest::new([Felt::from(4 as u8), Felt::ZERO, Felt::ZERO, Felt::ZERO])
    );

    println!("----->>> Showdown");
}

async fn bet(
    client: &mut AzeClient,
    player_account_id: AccountId,
    game_account_id: AccountId,
    faucet_account_id: AccountId,
    player_bet: u8,
    player_no: u8
) {
    fund_account(client, player_account_id, faucet_account_id).await;

    let game_account = client.get_account(game_account_id).unwrap().0;
    let game_account_storage = game_account.storage();

    let last_raiser = game_account_storage.get_item(RAISER_INDEX_SLOT);
    let last_phase_digest = game_account_storage.get_item(CURRENT_PHASE_SLOT);

    let player_index: u8 = FIRST_PLAYER_INDEX + PLAYER_STATS_SLOTS * (player_no - 1);

    let fungible_asset = FungibleAsset::new(faucet_account_id, BUY_IN_AMOUNT).unwrap();

    let playbet_txn_data = PlayBetTransactionData::new(
        Asset::Fungible(fungible_asset),
        player_account_id,
        game_account_id,
        player_bet
    );

    let transaction_template = AzeTransactionTemplate::PlayBet(playbet_txn_data);
    let txn_request = client.build_aze_play_bet_tx_request(transaction_template).unwrap();
    execute_tx_and_sync(client, txn_request.clone()).await;

    let note_id = txn_request.expected_output_notes()[0].id();
    let note = client.get_input_note(note_id).unwrap();

    let tx_template = TransactionTemplate::ConsumeNotes(game_account_id, vec![note.id()]);
    let tx_request = client.build_transaction_request(tx_template).unwrap();
    execute_tx_and_sync(client, tx_request).await;

    // update the game account storage
    let game_account = client.get_account(game_account_id).unwrap().0;
    let game_account_storage = game_account.storage();
    let player_index: u8 = (FIRST_PLAYER_INDEX + PLAYER_STATS_SLOTS * (player_no - 1));

    // check next player index
    assert_next_turn(&client, game_account_id, player_index, last_raiser, last_phase_digest).await;
    // check highest bet
    assert_eq!(
        game_account_storage.get_item(HIGHEST_BET_SLOT),
        RpoDigest::new([Felt::from(player_bet), Felt::ZERO, Felt::ZERO, Felt::ZERO])
    );
    // check player bet
    assert_eq!(
        game_account_storage.get_item((player_index + 3) as u8),
        RpoDigest::new([Felt::from(player_bet), Felt::ZERO, Felt::ZERO, Felt::ZERO])
    );
}

async fn check(
    client: &mut AzeClient,
    player_account_id: AccountId,
    game_account_id: AccountId,
    faucet_account_id: AccountId,
    player_no: u8
) {
    fund_account(client, player_account_id, faucet_account_id).await;

    let game_account = client.get_account(game_account_id).unwrap().0;
    let game_account_storage = game_account.storage();

    let last_raiser = game_account_storage.get_item(RAISER_INDEX_SLOT);
    let last_phase_digest = game_account_storage.get_item(CURRENT_PHASE_SLOT);

    let player_index: u8 = FIRST_PLAYER_INDEX + PLAYER_STATS_SLOTS * (player_no - 1);

    let fungible_asset = FungibleAsset::new(faucet_account_id, BUY_IN_AMOUNT).unwrap();

    let playcheck_txn_data = PlayCheckTransactionData::new(
        Asset::Fungible(fungible_asset),
        player_account_id,
        game_account_id
    );

    let transaction_template = AzeTransactionTemplate::PlayCheck(playcheck_txn_data);
    let txn_request = client.build_aze_play_check_tx_request(transaction_template).unwrap();
    execute_tx_and_sync(client, txn_request.clone()).await;

    let note_id = txn_request.expected_output_notes()[0].id();
    let note = client.get_input_note(note_id).unwrap();

    let tx_template = TransactionTemplate::ConsumeNotes(game_account_id, vec![note.id()]);
    let tx_request = client.build_transaction_request(tx_template).unwrap();
    execute_tx_and_sync(client, tx_request).await;

    // check next turn
    assert_next_turn(&client, game_account_id, player_index, last_raiser, last_phase_digest).await;
}

async fn fold(
    client: &mut AzeClient,
    player_account_id: AccountId,
    game_account_id: AccountId,
    faucet_account_id: AccountId,
    player_no: u8
) {
    fund_account(client, player_account_id, faucet_account_id).await;

    let game_account = client.get_account(game_account_id).unwrap().0;
    let game_account_storage = game_account.storage();

    let last_raiser = game_account_storage.get_item(RAISER_INDEX_SLOT);
    let last_phase_digest = game_account_storage.get_item(CURRENT_PHASE_SLOT);

    let player_index: u8 = FIRST_PLAYER_INDEX + PLAYER_STATS_SLOTS * (player_no - 1);
    let fold_index = player_index + IS_FOLD_OFFSET;

    let fungible_asset = FungibleAsset::new(faucet_account_id, BUY_IN_AMOUNT).unwrap();

    let playfold_txn_data = PlayFoldTransactionData::new(
        Asset::Fungible(fungible_asset),
        player_account_id,
        game_account_id
    );

    let transaction_template = AzeTransactionTemplate::PlayFold(playfold_txn_data);
    let txn_request = client.build_aze_play_fold_tx_request(transaction_template).unwrap();
    execute_tx_and_sync(client, txn_request.clone()).await;

    let note_id = txn_request.expected_output_notes()[0].id();
    let note = client.get_input_note(note_id).unwrap();

    let tx_template = TransactionTemplate::ConsumeNotes(game_account_id, vec![note.id()]);
    let tx_request = client.build_transaction_request(tx_template).unwrap();
    execute_tx_and_sync(client, tx_request).await;

    // update the game account storage
    let game_account = client.get_account(game_account_id).unwrap().0;
    let game_account_storage = game_account.storage();

    // check is_fold
    assert_eq!(
        game_account_storage.get_item(fold_index),
        RpoDigest::new([Felt::from(1 as u8), Felt::ZERO, Felt::ZERO, Felt::ZERO])
    );

    // check next turn index
    assert_next_turn(&client, game_account_id, player_index, last_raiser, last_phase_digest).await;
}

async fn call(
    client: &mut AzeClient,
    player_account_id: AccountId,
    game_account_id: AccountId,
    faucet_account_id: AccountId,
    player_no: u8
) {
    fund_account(client, player_account_id, faucet_account_id).await;

    let game_account = client.get_account(game_account_id).unwrap().0;
    let game_account_storage = game_account.storage();

    let last_raiser = game_account_storage.get_item(RAISER_INDEX_SLOT);
    let last_phase_digest = game_account_storage.get_item(CURRENT_PHASE_SLOT);

    let player_index: u8 = FIRST_PLAYER_INDEX + PLAYER_STATS_SLOTS * (player_no - 1);

    let fungible_asset = FungibleAsset::new(faucet_account_id, BUY_IN_AMOUNT).unwrap();

    let playcall_txn_data = PlayCallTransactionData::new(
        Asset::Fungible(fungible_asset),
        player_account_id,
        game_account_id
    );

    let transaction_template = AzeTransactionTemplate::PlayCall(playcall_txn_data);
    let txn_request = client.build_aze_play_call_tx_request(transaction_template).unwrap();
    execute_tx_and_sync(client, txn_request.clone()).await;

    let note_id = txn_request.expected_output_notes()[0].id();
    let note = client.get_input_note(note_id).unwrap();

    let tx_template = TransactionTemplate::ConsumeNotes(game_account_id, vec![note.id()]);
    let tx_request = client.build_transaction_request(tx_template).unwrap();
    execute_tx_and_sync(client, tx_request).await;

    // check next turn
    assert_next_turn(&client, game_account_id, player_index, last_raiser, last_phase_digest).await;
}

async fn raise(
    client: &mut AzeClient,
    player_account_id: AccountId,
    game_account_id: AccountId,
    faucet_account_id: AccountId,
    player_bet: u8,
    player_no: u8
) {
    fund_account(client, player_account_id, faucet_account_id).await;

    let game_account = client.get_account(game_account_id).unwrap().0;
    let game_account_storage = game_account.storage();

    let last_raiser = game_account_storage.get_item(RAISER_INDEX_SLOT);
    let last_phase_digest = game_account_storage.get_item(CURRENT_PHASE_SLOT);

    let fungible_asset = FungibleAsset::new(faucet_account_id, BUY_IN_AMOUNT).unwrap();

    let playraise_txn_data = PlayRaiseTransactionData::new(
        Asset::Fungible(fungible_asset),
        player_account_id,
        game_account_id,
        player_bet
    );

    let transaction_template = AzeTransactionTemplate::PlayRaise(playraise_txn_data);
    let txn_request = client.build_aze_play_raise_tx_request(transaction_template).unwrap();
    execute_tx_and_sync(client, txn_request.clone()).await;

    let note_id = txn_request.expected_output_notes()[0].id();
    let note = client.get_input_note(note_id).unwrap();

    let tx_template = TransactionTemplate::ConsumeNotes(game_account_id, vec![note.id()]);
    let tx_request = client.build_transaction_request(tx_template).unwrap();
    execute_tx_and_sync(client, tx_request).await;

    // update the game account storage
    let game_account = client.get_account(game_account_id).unwrap().0;
    let game_account_storage = game_account.storage();
    let player_index: u8 = (FIRST_PLAYER_INDEX + PLAYER_STATS_SLOTS * (player_no - 1));
    // check raiser
    assert_eq!(
        game_account_storage.get_item(RAISER_INDEX_SLOT),
        RpoDigest::new([
            Felt::from(player_index),
            Felt::ZERO,
            Felt::ZERO,
            Felt::ZERO,
        ])
    );
    // check current player index
    assert_next_turn(&client, game_account_id, player_index, last_raiser, last_phase_digest).await;
    // check highest bet
    assert_eq!(
        game_account_storage.get_item(HIGHEST_BET_SLOT),
        RpoDigest::new([Felt::from(player_bet), Felt::ZERO, Felt::ZERO, Felt::ZERO])
    );
    // check player bet
    assert_eq!(
        game_account_storage.get_item((player_index + 3) as u8),
        RpoDigest::new([Felt::from(player_bet), Felt::ZERO, Felt::ZERO, Felt::ZERO])
    );
}

async fn deal_card(
    client: &mut AzeClient,
    game_account_id: AccountId,
    player_account_id: AccountId,
    faucet_account_id: AccountId,
    card_number: u8
) {
    fund_account(client, game_account_id, faucet_account_id).await;

    let game_account = client.get_account(game_account_id).unwrap().0;
    let game_account_storage = game_account.storage();

    let fungible_asset = FungibleAsset::new(faucet_account_id, BUY_IN_AMOUNT).unwrap();

    let card_suit = 1u8;

    let input_cards = [
        [
            Felt::from(card_suit),
            Felt::from(card_number + 1),
            Felt::ZERO,
            Felt::ZERO,
        ],
        [
            Felt::from(card_suit),
            Felt::from(card_number + 2),
            Felt::ZERO,
            Felt::ZERO,
        ],
    ];

    let sendcard_txn_data = SendCardTransactionData::new(
        Asset::Fungible(fungible_asset),
        game_account_id,
        player_account_id,
        &input_cards
    );

    let transaction_template = AzeTransactionTemplate::SendCard(sendcard_txn_data);

    let txn_request = client.build_aze_send_card_tx_request(transaction_template).unwrap();
    execute_tx_and_sync(client, txn_request.clone()).await;

    let note_id = txn_request.expected_output_notes()[0].id();
    let note = client.get_input_note(note_id).unwrap();

    let tx_template = TransactionTemplate::ConsumeNotes(player_account_id, vec![note.id()]);
    let tx_request = client.build_transaction_request(tx_template).unwrap();
    execute_tx_and_sync(client, tx_request).await;
    assert_account_status(client, player_account_id, card_number as usize).await;
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
            Felt::from(slot_data.current_turn_index() + PLAYER_STATS_SLOTS),
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

    let player_bet = SMALL_BLIND_AMOUNT;
    slot_index = slot_index + 6;
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

    // check player balance
    assert_eq!(
        game_account_storage.get_item(PLAYER_BALANCE_SLOT),
        RpoDigest::new([Felt::from(remaining_balance), Felt::ZERO, Felt::ZERO, Felt::ZERO])
    );
}

async fn assert_slot_status_fold(
    client: &AzeClient,
    account_id: AccountId,
    slot_data: GameStorageSlotData
) {
    let (account, _) = client.get_account(account_id).unwrap();
    let game_account_storage = account.storage();

    let fold_index = slot_data.current_turn_index() + IS_FOLD_OFFSET;

    // check is_fold
    assert_eq!(
        game_account_storage.get_item(fold_index),
        RpoDigest::new([Felt::from(1 as u8), Felt::ZERO, Felt::ZERO, Felt::ZERO])
    );

    let next_turn_index = slot_data.current_turn_index() + PLAYER_STATS_SLOTS;
    // check next turn index
    assert_eq!(
        game_account_storage.get_item(CURRENT_TURN_INDEX_SLOT),
        RpoDigest::new([Felt::from(next_turn_index), Felt::ZERO, Felt::ZERO, Felt::ZERO])
    );
}

async fn assert_slot_status_check(
    client: &AzeClient,
    account_id: AccountId,
    slot_data: GameStorageSlotData,
    player_number: u8
) {
    let (account, _) = client.get_account(account_id).unwrap();
    let game_account_storage = account.storage();

    // assert check count
    let check_count = game_account_storage.get_item(CHECK_COUNTER_SLOT);
    assert_eq!(check_count, RpoDigest::new([Felt::from(player_number as u8), Felt::ZERO, Felt::ZERO, Felt::ZERO]));

    let next_turn_index = slot_data.current_turn_index() + PLAYER_STATS_SLOTS * player_number;
    // check next turn index
    assert_eq!(
        game_account_storage.get_item(CURRENT_TURN_INDEX_SLOT),
        RpoDigest::new([Felt::from(next_turn_index), Felt::ZERO, Felt::ZERO, Felt::ZERO])
    );
}

async fn fund_account(client: &mut AzeClient, account_id: AccountId, faucet_account_id: AccountId) {
    let note = mint_note(client, account_id, faucet_account_id, NoteType::Public).await;
    consume_notes(client, account_id, &[note]).await;
}

async fn log_slots(client: &AzeClient, account_id: AccountId) {
    let (regular_account, _seed) = client.get_account(account_id).unwrap();
    for i in 1..117 {
        println!("Account slot {:?} --> {:?}", i, regular_account.storage().get_item(i));
    }
}

async fn assert_next_turn(client: &AzeClient, account_id: AccountId, player_index: u8, last_raiser_index: RpoDigest, last_phase_digest: RpoDigest) {
    let (account, _) = client.get_account(account_id).unwrap();
    let game_account_storage = account.storage();
    log_slots(client, account_id).await;

    let mut next_player_index = if player_index == LAST_PLAYER_INDEX {
        FIRST_PLAYER_INDEX
    } else {
        player_index + PLAYER_STATS_SLOTS
    };

    // If phase was increased, then next player should be the first player
    let mut last_phase = 0;
    while RpoDigest::new([Felt::from(last_phase as u8), Felt::ZERO, Felt::ZERO, Felt::ZERO]) != last_phase_digest {
        last_phase += 1;
    }

    if RpoDigest::new([Felt::from(last_phase as u8 + 1), Felt::ZERO, Felt::ZERO, Felt::ZERO]) == game_account_storage.get_item(CURRENT_PHASE_SLOT) {
        next_player_index = FIRST_PLAYER_INDEX;
    }

    // find next player which has not folded
    while game_account_storage.get_item(next_player_index + IS_FOLD_OFFSET) == RpoDigest::new([Felt::from(1 as u8), Felt::ZERO, Felt::ZERO, Felt::ZERO]) {
        if next_player_index == player_index {
            break;
        }

        next_player_index = next_player_index + PLAYER_STATS_SLOTS;
        if next_player_index > LAST_PLAYER_INDEX {
            next_player_index = FIRST_PLAYER_INDEX;
        }
    }

    assert_eq!(
        game_account_storage.get_item(CURRENT_TURN_INDEX_SLOT),
        RpoDigest::new([Felt::from(next_player_index as u8), Felt::ZERO, Felt::ZERO, Felt::ZERO])
    );
}