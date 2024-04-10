use crate::utils::create_aze_store_path;
use miden_client::client::rpc::NodeRpcClient;
use miden_client::{
    client::{
        accounts::{AccountStorageMode, AccountTemplate},
        rpc::TonicRpcClient,
        transactions::{PaymentTransactionData, TransactionTemplate},
        Client,
    },
    config::{ClientConfig, RpcConfig},
    errors::{ClientError, NodeRpcClientError},
    store::{sqlite_store::SqliteStore, NoteFilter, Store, TransactionFilter},
};
use miden_tx::DataStore;

type AzeClient = Client<TonicRpcClient, SqliteStore>;

pub trait AzeGameMethods {
    fn new_aze_game_account(&mut self, template: AccountTemplate);
    fn new_aze_player_account(&mut self, template: AccountTemplate);
}

pub enum AzeAccountTemplate {
    PlayerAccount {
        mutable_code: bool,
        storage_mode: AccountStorageMode,
    },
    GameAccount {
        // need to change it and he would need to pass whole game storage
        mutable_code: bool,
        storage_mode: AccountStorageMode,
    },
}

fn create_aze_client() -> AzeClient {
    let client_config = ClientConfig {
        store: create_aze_store_path()
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
    AzeClient::new(TonicRpcClient::new(&rpc_endpoint), store, executor_store).unwrap()
}

// impl<N: NodeRpcClient, D: Store> AzeGameMethods for Client<N, D> {
//     fn new_aze_account() {
//         println!("Creating new account");
//     }
// }
