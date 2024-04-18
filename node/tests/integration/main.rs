use aze_lib::client::AzeAccountTemplate;
use aze_lib::client::{AzeClient, AzeGameMethods};
use aze_lib::constants::BUY_IN_AMOUNT;
use aze_lib::executor::execute_tx_and_sync;
use aze_lib::utils::{get_random_coin, load_config};
use miden_client::client::accounts::AccountStorageMode;
use miden_client::{
    client::rpc::TonicRpcClient,
    config::{ClientConfig, RpcConfig},
    errors::{ClientError, NodeRpcClientError},
    store::sqlite_store::SqliteStore,
};
use miden_crypto::hash::rpo::RpoDigest;
use miden_crypto::FieldElement;
use miden_objects::Felt;
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

    // TODO: create game account and check the storage is correctly assigned
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
