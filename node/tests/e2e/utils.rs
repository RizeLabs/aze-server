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
    IS_FOLD_OFFSET,
    PLAYER_BET_OFFSET,
    FIRST_PLAYER_INDEX,
    LAST_PLAYER_INDEX,
    HIGHEST_BET,
    PLAYER_INITIAL_BALANCE,
    CURRENT_TURN_INDEX_SLOT,
    RAISER_INDEX_SLOT,
    PLAYER_STATS_SLOTS,
    HIGHEST_BET_SLOT,
    CURRENT_PHASE_SLOT,
    PLAYER_CARD1_SLOT,
    PLAYER_CARD2_SLOT
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
use ansi_term::Colour::{ Green, Yellow };

pub fn create_test_client() -> AzeClient {
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

pub fn setup_accounts(
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

pub async fn fund_account(client: &mut AzeClient, account_id: AccountId, faucet_account_id: AccountId) {
    let fungible_asset = FungibleAsset::new(faucet_account_id, 10 * BUY_IN_AMOUNT).unwrap();
    let tx_template = TransactionTemplate::MintFungibleAsset(
        fungible_asset,
        account_id,
        NoteType::Public
    );
    let tx_request = client.build_transaction_request(tx_template).unwrap();
    let _ = execute_tx_and_sync(client, tx_request.clone()).await;
    let note_id = tx_request.expected_output_notes()[0].id();
    let note = client.get_input_note(note_id).unwrap();
    consume_notes(client, account_id, &[note.try_into().unwrap()]).await;
    println!("{}", Yellow.paint(format!("Funded account: {:?}", account_id)));
}

pub async fn bet(
    client: &mut AzeClient,
    player_account_id: AccountId,
    game_account_id: AccountId,
    faucet_account_id: AccountId,
    player_bet: u8,
    player_no: u8
) {
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
        game_account_storage.get_item((player_index + PLAYER_BET_OFFSET) as u8),
        RpoDigest::new([Felt::from(player_bet), Felt::ZERO, Felt::ZERO, Felt::ZERO])
    );
}

pub async fn check(
    client: &mut AzeClient,
    player_account_id: AccountId,
    game_account_id: AccountId,
    faucet_account_id: AccountId,
    player_no: u8
) {
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

pub async fn fold(
    client: &mut AzeClient,
    player_account_id: AccountId,
    game_account_id: AccountId,
    faucet_account_id: AccountId,
    player_no: u8
) {
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

pub async fn call(
    client: &mut AzeClient,
    player_account_id: AccountId,
    game_account_id: AccountId,
    faucet_account_id: AccountId,
    player_no: u8
) {
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

pub async fn raise(
    client: &mut AzeClient,
    player_account_id: AccountId,
    game_account_id: AccountId,
    faucet_account_id: AccountId,
    player_bet: u8,
    player_no: u8
) {
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
        game_account_storage.get_item((player_index + PLAYER_BET_OFFSET) as u8),
        RpoDigest::new([Felt::from(player_bet), Felt::ZERO, Felt::ZERO, Felt::ZERO])
    );
}

pub async fn deal_card(
    client: &mut AzeClient,
    game_account_id: AccountId,
    player_account_id: AccountId,
    faucet_account_id: AccountId,
    card_number: u8
) {
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
    // check player cards
    let (account, _) = client.get_account(player_account_id).unwrap();
    assert_eq!(
        account.storage().get_item(PLAYER_CARD1_SLOT),
        RpoDigest::new([
            Felt::from(card_suit),
            Felt::from(card_number + 1),
            Felt::ZERO,
            Felt::ZERO,
        ])
    );
    assert_eq!(
        account.storage().get_item(PLAYER_CARD2_SLOT),
        RpoDigest::new([
            Felt::from(card_suit),
            Felt::from(card_number + 2),
            Felt::ZERO,
            Felt::ZERO,
        ])
    );
}

async fn assert_next_turn(client: &AzeClient, account_id: AccountId, player_index: u8, last_raiser_index: RpoDigest, last_phase_digest: RpoDigest) {
    let (account, _) = client.get_account(account_id).unwrap();
    let game_account_storage = account.storage();

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