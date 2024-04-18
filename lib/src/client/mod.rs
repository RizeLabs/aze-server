use crate::accounts::{create_basic_aze_game_account, create_basic_aze_player_account};
use crate::utils::{create_aze_store_path, load_config};
use crate::notes::create_send_card_note;
use crate::constants::CLIENT_CONFIG_FILE_NAME;
use miden_client::client::rpc::NodeRpcClient;
use miden_client::{client, store};
use miden_client::store::data_store::{self, ClientDataStore};
extern crate alloc;
use alloc::collections::{BTreeMap, BTreeSet};

use miden_client::{
    client::{
        accounts::{AccountStorageMode, AccountTemplate},
        get_random_coin,
        rpc::TonicRpcClient,
        transactions::transaction_request::TransactionRequest,
        transactions::transaction_request,
        Client,
    },
    config::{ClientConfig, RpcConfig},
    errors::{ClientError, NodeRpcClientError},
    store::{sqlite_store::SqliteStore, NoteFilter, Store, TransactionFilter, AuthInfo},
};

use miden_lib::AuthScheme;
use miden_objects::crypto::rand::FeltRng;
use miden_objects::notes::NoteType;
use miden_objects::{
    accounts::{Account, AccountData, AccountId, AccountStub, AccountType, AuthData},
    assets::TokenSymbol,
    assembly::ProgramAst,
    crypto::dsa::rpo_falcon512::SecretKey,
    Felt, Word,
};
use miden_objects::crypto::rand::RpoRandomCoin;
use miden_objects::assets::Asset;
use miden_tx::{DataStore, TransactionExecutor};
use rand::{rngs::ThreadRng, Rng};

pub type AzeClient = Client<TonicRpcClient, RpoRandomCoin, SqliteStore>;

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
    // fn get_tx_executor(&self) -> TransactionExecutor<ClientDataStore<D>>;
    fn store(&self) -> SqliteStore;
    fn get_random_coin(&self) -> RpoRandomCoin;
    fn new_send_card_transaction(
        &mut self,
        asset: Asset,
        sender_account_id: AccountId,
        target_account_id: AccountId,
        cards: &[Felt; 4],
    ) -> Result<(), ClientError>;
    fn build_aze_send_card_tx_request(
        &mut self, 
        // auth_info: AuthInfo,
        transaction_template: AzeTransactionTemplate,
    ) -> Result<TransactionRequest, ClientError>;
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
    let rng = get_random_coin();

    let rpc_endpoint = client_config.rpc.endpoint.to_string();
    let store = SqliteStore::new((&client_config).into()).unwrap();
    let executor_store = SqliteStore::new((&client_config).into()).unwrap();
    AzeClient::new(TonicRpcClient::new(&rpc_endpoint), rng, store, executor_store).unwrap()
}

impl<N: NodeRpcClient, R: FeltRng, S: Store> AzeGameMethods for Client<N, R, S> {
    
    // fn get_tx_executor(&self) -> TransactionExecutor<ClientDataStore<D>> {
    //     self
    // }
    
    fn store(&self) -> SqliteStore {
        let mut current_dir = std::env::current_dir().map_err(|err| err.to_string()).unwrap();
        current_dir.push(CLIENT_CONFIG_FILE_NAME);
        let client_config = load_config(current_dir.as_path()).unwrap();
    
        let rpc_endpoint = client_config.rpc.endpoint.to_string();
        let store = SqliteStore::new((&client_config).into()).unwrap();
        let executor_store = SqliteStore::new((&client_config).into()).unwrap();
        executor_store
    }

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

        let key_pair = SecretKey::with_rng(rng);

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

        let key_pair = SecretKey::with_rng(rng);

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
    // TODO: include note_type as an argument here for now we are hardcoding it
    fn build_aze_send_card_tx_request(
        &mut self,
        // auth_info: AuthInfo,
        transaction_template: AzeTransactionTemplate,
    ) -> Result<TransactionRequest, ClientError> {

        let account_id = transaction_template.account_id();
        let account_auth = self.store().get_account_auth(account_id)?;

        let (sender_account_id, target_account_id, cards, asset) = match transaction_template {
            AzeTransactionTemplate::SendCard(SendCardTransactionData {
                asset,
                sender_account_id,
                target_account_id,
                cards,
            }) => (sender_account_id, target_account_id, cards, asset),
        };

        let random_coin = self.get_random_coin();

        let created_note = create_send_card_note(
            sender_account_id,
            target_account_id,
            [asset].to_vec(),
            NoteType::Public,
            random_coin,
            cards
        )?;

        let recipient = created_note
            .recipient_digest()
            .iter()
            .map(|x| x.as_int().to_string())
            .collect::<Vec<_>>()
            .join(".");
    
        let note_tag = created_note.metadata().tag().inner();

        // TODO: remove this hardcoded note type
        let note_type = NoteType::Public;

        let tx_script = ProgramAst::parse(
            &transaction_request::AUTH_SEND_ASSET_SCRIPT
                .replace("{recipient}", &recipient)
                .replace("{note_type}", &Felt::new(note_type as u64).to_string())
                .replace("{tag}", &Felt::new(note_tag.into()).to_string())
                .replace("{asset}", &prepare_word(&asset.into()).to_string()),
        ) .expect("shipped MASM is well-formed");

        let tx_script = {
            let script_inputs = vec![account_auth.into_advice_inputs()];
            self.compile_tx_script(tx_script, script_inputs, vec![])?
        };

        println!("Created txn script");

        Ok(TransactionRequest::new(
            sender_account_id,
            BTreeMap::new(),
            vec![created_note],
            Some(tx_script),
        ))

        // let block_num = client.store.get_sync_height();


        // let execution_store = self.store();

        // let mut executor = TransactionExecutor::new(ClientDataStore::new(execution_store));

        // let data_store =
        //      DataStore::with_existing(Some(target_account.clone()), Some(vec![created_note.clone()]));
        
        // let mock_data_store = 

       





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

pub(crate) fn prepare_word(word: &Word) -> String {
    word.iter().map(|x| x.as_int().to_string()).collect::<Vec<_>>().join(".")
}