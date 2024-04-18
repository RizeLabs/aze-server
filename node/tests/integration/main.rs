use aze_lib::client::{AzeClient, AzeGameMethods, AzeAccountTemplate, AzeTransactionTemplate, SendCardTransactionData};
use aze_lib::constants::BUY_IN_AMOUNT;
use aze_lib::executor::execute_tx_and_sync;
use aze_lib::utils::{get_random_coin, load_config};
use miden_client::client::accounts::{AccountStorageMode, AccountTemplate};
use miden_client::{
    client::rpc::TonicRpcClient,
    config::{ClientConfig, RpcConfig},
    errors::{ClientError, NodeRpcClientError},
    store::sqlite_store::SqliteStore,
};
use miden_crypto::hash::rpo::RpoDigest;
use miden_crypto::FieldElement;
use miden_objects::{
    Felt,
    assets::{TokenSymbol, FungibleAsset, Asset}
};
use std::{env::temp_dir, time::Duration};
// use uuid::Uuid;

fn create_test_client() -> AzeClient {
    let client_config = ClientConfig {
        store: create_test_store_path()
            .into_os_string()
            .into_string()
            .unwrap()
            .try_into()
            .unwrap(),
        rpc: RpcConfig::default(),
    };

    println!("Client Config: {:?}", client_config);

    let rpc_endpoint = client_config.rpc.endpoint.to_string();
    let store = SqliteStore::new((&client_config).into()).unwrap();
    let executor_store = SqliteStore::new((&client_config).into()).unwrap();
    let rng = get_random_coin();
    AzeClient::new(
        TonicRpcClient::new(&rpc_endpoint),
        rng,
        store,
        executor_store,
    )
    .unwrap()
}

fn create_test_store_path() -> std::path::PathBuf {
    let mut temp_file = temp_dir();
    temp_file.push(format!("{}.sqlite3", "some-random"));
    temp_file
}

async fn wait_for_node(client: &mut AzeClient) {
    const NODE_TIME_BETWEEN_ATTEMPTS: u64 = 5;
    const NUMBER_OF_NODE_ATTEMPTS: u64 = 60;

    println!("Waiting for Node to be up. Checking every {NODE_TIME_BETWEEN_ATTEMPTS}s for {NUMBER_OF_NODE_ATTEMPTS} tries...");

    for _try_number in 0..NUMBER_OF_NODE_ATTEMPTS {
        match client.sync_state().await {
            Err(ClientError::NodeRpcClientError(NodeRpcClientError::ConnectionError(_))) => {
                std::thread::sleep(Duration::from_secs(NODE_TIME_BETWEEN_ATTEMPTS));
            }
            Err(other_error) => {
                panic!("Unexpected error: {other_error}");
            }
            _ => return,
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

    // TODO: somehow manage the game seed as well
    let (game_account, _) = client
        .new_game_account(AzeAccountTemplate::GameAccount {
            mutable_code: false,
            storage_mode: AccountStorageMode::Local,
        })
        .unwrap();
    let game_account_storage = game_account.storage();

    let mut slot_index = 1;

    // check are the cards has been correctly placed
    for card_suit in 1..5 {
        for card_number in 1..13 {
            let slot_item = RpoDigest::new([
                Felt::from(card_suit as u8),
                Felt::from(card_number as u8),
                Felt::ZERO, // denotes is encrypted
                Felt::ZERO,
            ]);

            assert_eq!(
                game_account_storage.get_item(slot_index),
                slot_item,
            );

            slot_index = slot_index + 1;
        }
    }

    // checking next turn 
    assert_eq!(
        game_account_storage.get_item(slot_index),
        RpoDigest::new([Felt::ZERO, Felt::ZERO, Felt::ZERO, Felt::ZERO]),
    );

    slot_index = slot_index + 1;

    // checking the small blind amount
    assert_eq!(
        game_account_storage.get_item(slot_index),
        RpoDigest::new([Felt::from(small_blind_amt), Felt::ZERO, Felt::ZERO, Felt::ZERO]),
    );

    slot_index = slot_index + 1;

    // checking the big blind amount
    assert_eq!(
        game_account_storage.get_item(slot_index),
        RpoDigest::new([Felt::from(small_blind_amt * 2), Felt::ZERO, Felt::ZERO, Felt::ZERO]),
    );

    slot_index = slot_index + 1;

    // checking the buy in amount
    assert_eq!(
        game_account_storage.get_item(slot_index),
        RpoDigest::new([Felt::from(buy_in_amt), Felt::ZERO, Felt::ZERO, Felt::ZERO]),
    );

    slot_index = slot_index + 1;
    // checking no of player slot 
    assert_eq!(
        game_account_storage.get_item(slot_index),
        RpoDigest::new([Felt::from(no_of_players), Felt::ZERO, Felt::ZERO, Felt::ZERO]),
    );

    slot_index = slot_index + 1;
    // checking flop index slot
    assert_eq!(
        game_account_storage.get_item(slot_index),
        RpoDigest::new([Felt::from(flop_index), Felt::ZERO, Felt::ZERO, Felt::ZERO]),
    );
    
}

#[tokio::test]
async fn test_cards_distribution() {
    let mut client: AzeClient = create_test_client();

    let (game_account, _) = client
        .new_game_account(AzeAccountTemplate::GameAccount {
            mutable_code: false,
            storage_mode: AccountStorageMode::Local,
        })
        .unwrap();

    let game_account_storage = game_account.storage();

    // TODO: for now we''ll distribute cards to two players 
    let (player1_account, _) = client
        .new_account(AccountTemplate::BasicWallet { 
            mutable_code: false, 
            storage_mode: AccountStorageMode::Local 
        }).unwrap();

    let (player2_account, _) = client
        .new_account(AccountTemplate::BasicWallet { 
            mutable_code: false, 
            storage_mode: AccountStorageMode::Local 
        }).unwrap();

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
    let fungible_asset = FungibleAsset::new(faucet_account_id, BUY_IN_AMOUNT).unwrap();

    let player_account_ids = vec![player1_account.id(), player2_account.id()];

    let mut cards: Vec<[Felt; 4]> = vec![];

    for slot_index in 1..2 * player_account_ids.len() + 1 {
        let slot_item = game_account_storage.get_item(slot_index as u8);
        cards.push(slot_item.into());
    }

    println!("Card {:?}", cards);

    // TODO: fix below test
    // println!("Start sending cards to players");
    // for (i, _) in player_account_ids.iter().enumerate() {
    //     let target_account_id = player_account_ids[i];
    //     println!("Target account id {:?}", target_account_id);

    //     let input_cards = [cards[i], cards[i + 1]]; // don't you think the input cards should contain 8 felt -> 2 cards
    //     let sendcard_txn_data = SendCardTransactionData::new(
    //         Asset::Fungible(fungible_asset),
    //         game_account.id(),
    //         target_account_id,
    //         &input_cards,
    //     );

    //     let transaction_template = AzeTransactionTemplate::SendCard(sendcard_txn_data);

    //     let txn_request = client
    //         .build_aze_send_card_tx_request(transaction_template)
    //         .unwrap();

    //     execute_tx_and_sync(&mut client, txn_request).await;
    //     println!("Executed and synced with node");
    // }

}