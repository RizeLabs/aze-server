use crate::accounts::{create_basic_aze_game_account, create_basic_aze_player_account};
use crate::utils::{create_aze_store_path, load_config};
use crate::constants::CLIENT_CONFIG_FILE_NAME;
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
    store::{sqlite_store::SqliteStore, NoteFilter, Store, TransactionFilter, AuthInfo},
};
use miden_lib::AuthScheme;
use miden_objects::crypto::rand::FeltRng;
use miden_objects::{
    accounts::{Account, AccountData, AccountId, AccountStub, AccountType, AuthData},
    assets::TokenSymbol,
    crypto::dsa::rpo_falcon512::KeyPair,
    Felt, Word,
};
use miden_objects::crypto::rand::RpoRandomCoin;
use miden_objects::assets::Asset;
use miden_tx::DataStore;
use rand::{rngs::ThreadRng, Rng};

pub type AzeClient = Client<TonicRpcClient, SqliteStore>;

#[derive(Clone)]
pub struct SendCardTransactionData {
    asset: Asset,
    sender_account_id: AccountId,
    target_account_id: AccountId,
    cards: [Felt; 4],
}

impl SendCardTransactionData {
    pub fn account_id(&self) -> AccountId {
        self.sender_account_id
    }
    pub fn new(asset: Asset, sender_account_id: AccountId, target_account_id: AccountId, cards: &[Felt; 4]) -> Self {
        Self {
            asset,
            sender_account_id,
            target_account_id,
            cards: *cards,
        }
    }
}

pub trait AzeGameMethods {
    fn get_random_coin(&self) -> RpoRandomCoin;
    fn new_send_card_transaction(
        &mut self,
        asset: Asset,
        sender_account_id: AccountId,
        target_account_id: AccountId,
        cards: &[Felt; 4],
    ) -> Result<(), ClientError>;
    fn new_aze_send_card_transaction(
        &mut self, 
        transaction_template: AzeTransactionTemplate,
        client: &mut AzeClient,
    ) -> Result<(), ClientError>;
    fn new_game_account(
        &mut self,
        template: AzeAccountTemplate,
    ) -> Result<(Account, Word), ClientError>;
    fn new_aze_game_account(
        &mut self,
        mutable_code: bool,
        rng: &mut ThreadRng,
        account_storage_mode: AccountStorageMode,
    ) -> Result<(Account, Word), ClientError>;
    fn new_aze_player_account(
        &mut self,
        mutable_code: bool,
        rng: &mut ThreadRng,
        account_storage_mode: AccountStorageMode,
    ) -> Result<(Account, Word), ClientError>;
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

pub fn create_aze_client() -> AzeClient {
    // let client_config = ClientConfig {
    //     store: create_aze_store_path()
    //         .into_os_string()
    //         .into_string()
    //         .unwrap()
    //         .try_into()
    //         .unwrap(),
    //     rpc: RpcConfig::default(),
    // };

    let mut current_dir = std::env::current_dir().map_err(|err| err.to_string()).unwrap();
    current_dir.push(CLIENT_CONFIG_FILE_NAME);
    let client_config = load_config(current_dir.as_path()).unwrap();

    let rpc_endpoint = client_config.rpc.endpoint.to_string();
    let store = SqliteStore::new((&client_config).into()).unwrap();
    let executor_store = SqliteStore::new((&client_config).into()).unwrap();
    AzeClient::new(TonicRpcClient::new(&rpc_endpoint), store, executor_store).unwrap()
}

impl<N: NodeRpcClient, D: Store> AzeGameMethods for Client<N, D> {
    fn new_game_account(
        &mut self,
        template: AzeAccountTemplate,
    ) -> Result<(Account, Word), ClientError> {
        let mut rng = rand::thread_rng();

        let account_and_seed = match template {
            AzeAccountTemplate::PlayerAccount {
                mutable_code,
                storage_mode,
            } => self.new_aze_player_account(mutable_code, &mut rng, storage_mode),
            AzeAccountTemplate::GameAccount {
                mutable_code,
                storage_mode,
            } => self.new_aze_game_account(mutable_code, &mut rng, storage_mode),
        }?;

        Ok(account_and_seed)
    }

    fn new_aze_game_account(
        &mut self,
        mutable_code: bool, // will remove it later on
        rng: &mut ThreadRng,
        account_storage_mode: AccountStorageMode,
    ) -> Result<(Account, Word), ClientError> {
        if let AccountStorageMode::OnChain = account_storage_mode {
            todo!("Recording the account on chain is not supported yet");
        }

        let key_pair: KeyPair = KeyPair::new()?;

        let auth_scheme: AuthScheme = AuthScheme::RpoFalcon512 {
            pub_key: key_pair.public_key(),
        };

        // we need to use an initial seed to create the wallet account
        let init_seed: [u8; 32] = rng.gen();

        let (account, seed) = create_basic_aze_game_account(
            init_seed,
            auth_scheme,
            AccountType::RegularAccountImmutableCode,
        ).unwrap();

        // will do insert account later on since there is some type mismatch due to miden object crate
        self.insert_account(&account, Some(seed), &AuthInfo::RpoFalcon512(key_pair))?;
        Ok((account, seed))
    }

    fn new_aze_player_account(
        &mut self,
        mutable_code: bool,
        rng: &mut ThreadRng,
        account_storage_mode: AccountStorageMode,
    ) -> Result<(Account, Word), ClientError> {
        if let AccountStorageMode::OnChain = account_storage_mode {
            todo!("Recording the account on chain is not supported yet");
        }

        let key_pair: KeyPair = KeyPair::new()?;

        let auth_scheme: AuthScheme = AuthScheme::RpoFalcon512 {
            pub_key: key_pair.public_key(),
        };

        // we need to use an initial seed to create the wallet account
        let init_seed: [u8; 32] = rng.gen();

        let (account, seed) = create_basic_aze_player_account(
            init_seed,
            auth_scheme,
            AccountType::RegularAccountImmutableCode,
        ).unwrap();

        // will do insert account later on since there is some type mismatch due to miden object crate
        self.insert_account(&account, Some(seed), &AuthInfo::RpoFalcon512(key_pair))?;
        Ok((account, seed))
    }

    // fn new_aze_transaction(
    //     sender_account_id: AccountId,
    //     receiver_account_id: AccountId,
    //     assets: Vec<Asset>,
    //     mut rng: FeltRng
    // ) {
    //     let new_note = create_deal_note(sender_account_id, receiver_account_id, assets, rng).unwrap();

    // }
    fn new_aze_send_card_transaction(
        &mut self,
        transaction_template: AzeTransactionTemplate,
        client: &mut AzeClient,
    ) -> Result<(), ClientError> {

        // match transaction_template {
        //     AzeTransactionTemplate::SendCard(AzeTransactionTemplate {
        //         asset: fungible_asset,
        //         sender_account_id,
        //         target_account_id,
        //         cards,
        //     }) => self.new_send_card_transaction(fungible_asset, sender_account_id, target_account_id, cards),

        // };


        // let created_note = create_send_card_note(
        //     sender_account_id,
        //     target_account_id,
        //     transaction_template.
        //     vec![fungible_asset],
        //     random_coin,
        // )?;


        // client.new_transaction(transaction_template)


         Ok(())

    }


    fn new_send_card_transaction(
        &mut self,
        asset: Asset,
        sender_account_id: AccountId,
        target_account_id: AccountId,
        cards: &[Felt; 4],
    ) -> Result<(), ClientError> {
        // let random_coin = 
        Ok(())    
    }

    fn get_random_coin(&self) -> RpoRandomCoin {
        // TODO: Initialize coin status once along with the client and persist status for retrieval
        let mut rng = rand::thread_rng();
        let coin_seed: [u64; 4] = rng.gen();

        RpoRandomCoin::new(coin_seed.map(Felt::new))
    }
}

//implement a new transaction template
pub enum AzeTransactionTemplate {
    SendCard(SendCardTransactionData),
}

impl AzeTransactionTemplate {
    //returns the executor account id
    pub fn account_id(&self) -> AccountId {
        match self{
            AzeTransactionTemplate::SendCard(p) => p.account_id(),
        }
    }

}
