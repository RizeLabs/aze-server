use miden_objects::{
    accounts::{ Account, AccountCode, AccountId, AccountStorage, SlotItem },
    assembly::{ ModuleAst, ProgramAst },
    assets::{ Asset, AssetVault, FungibleAsset },
    crypto::{
        dsa::rpo_falcon512::SecretKey,
        utils::Serializable,
        rand::FeltRng,
        rand::RpoRandomCoin,
    },
    notes::{ Note, NoteId, NoteScript },
    transaction::{
        ChainMmr,
        ExecutedTransaction,
        InputNote,
        InputNotes,
        ProvenTransaction,
        TransactionInputs,
    },
    BlockHeader,
    Felt,
    Word,
};
use std::{ env::temp_dir, fs, time::Duration };
use miden_client::{
    client::{ rpc::NodeRpcClient, Client },
    config::ClientConfig,
    errors::{ ClientError, NoteIdPrefixFetchError },
    store::{ sqlite_store::SqliteStore, InputNoteRecord, NoteFilter as ClientNoteFilter, Store },
};
use std::path::Path;
use figment::{ providers::{ Format, Toml }, Figment };
use ::rand::Rng;
use crate::client::AzeClient;

// use uuid::Uuid;

pub fn get_new_key_pair_with_advice_map() -> (Word, Vec<Felt>) {
    let keypair = SecretKey::new();

    let pk: Word = keypair.public_key().into();
    let pk_sk_bytes = keypair.to_bytes();
    let pk_sk_felts: Vec<Felt> = pk_sk_bytes
        .iter()
        .map(|a| Felt::new(*a as u64))
        .collect::<Vec<Felt>>();

    (pk, pk_sk_felts)
}

pub fn create_aze_store_path() -> std::path::PathBuf {
    let mut temp_file = temp_dir();
    temp_file.push(format!("{}.sqlite3", "random")); // for now don't know why uuid is not importing
    temp_file
}

pub fn load_config(config_file: &Path) -> Result<ClientConfig, String> {
    Figment::from(Toml::file(config_file))
        .extract()
        .map_err(|err| format!("Failed to load {} config file: {err}", config_file.display()))
}

pub fn get_random_coin() -> RpoRandomCoin {
    // TODO: Initialize coin status once along with the client and persist status for retrieval
    let mut rng = rand::thread_rng();
    let coin_seed: [u64; 4] = rng.gen();

    RpoRandomCoin::new(coin_seed.map(Felt::new))
}

// TODO hide this methods under debug feature
pub async fn log_account_status(client: &AzeClient, account_id: AccountId) {
    let (regular_account, _seed) = client.get_account(account_id).unwrap();
    println!("Account asset count --> {:?}", regular_account.vault().assets().count());
    println!("Account storage root --> {:?}", regular_account.storage().root());
    println!("Account slot 100 --> {:?}", regular_account.storage().get_item(100));
    println!("Account slot 101 --> {:?}", regular_account.storage().get_item(101));
}

pub async fn log_slots(client: &AzeClient, account_id: AccountId) {
    let (regular_account, _seed) = client.get_account(account_id).unwrap();
    for i in 1..100 {
        println!("Account slot {:?} --> {:?}", i, regular_account.storage().get_item(i));
    }
}
