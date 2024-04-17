use aze_lib::client::{AzeClient, AzeGameMethods};
use aze_lib::executor::execute_tx_and_sync;
use aze_lib::constants::BUY_IN_AMOUNT;
use aze_lib::client::AzeAccountTemplate;
use aze_lib::utils::get_random_coin;
use miden_client::client::accounts::AccountStorageMode;
use miden_client::{ 
    client:: {
    rpc::TonicRpcClient,
    },
    config::{ClientConfig, RpcConfig},
    errors::{ClientError, NodeRpcClientError},
    store::{sqlite_store::SqliteStore}
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

    let rpc_endpoint = client_config.rpc.endpoint.to_string();
    let store = SqliteStore::new((&client_config).into()).unwrap();
    let executor_store = SqliteStore::new((&client_config).into()).unwrap();
    let rng = get_random_coin();
    AzeClient::new(TonicRpcClient::new(&rpc_endpoint), rng, store, executor_store).unwrap()
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
            },
            Err(other_error) => {
                panic!("Unexpected error: {other_error}");
            },
            _ => return,
        }
    }

    panic!("Unable to connect to node");
}

#[tokio::test]
async fn test_create_aze_game_account() {
    let mut client = create_test_client();
    // TODO: create game account and check the storage is correctly assigned 
    let (game_account, _) = client.new_game_account(AzeAccountTemplate::GameAccount { mutable_code: false, storage_mode: AccountStorageMode::Local }).unwrap();
}