use miden_lib::transaction::TransactionKernel;
use miden_objects::{
    accounts::{ Account, AccountCode, AccountId, AccountStorage, StorageSlotType },
    assembly::{ ModuleAst, ProgramAst },
    assets::{ Asset, AssetVault, FungibleAsset },
    crypto::rand::{ FeltRng, RpoRandomCoin },
    notes::{
        Note,
        NoteAssets,
        NoteExecutionMode,
        NoteInputs,
        NoteMetadata,
        NoteRecipient,
        NoteScript,
        NoteTag,
        NoteType,
    },
    transaction::{ TransactionArgs, InputNote },
    Felt,
    NoteError,
    Word,
    ZERO,
};
use miden_tx::TransactionExecutor;
use miden_client::{
    client::{
        rpc::NodeRpcClient,
        transactions::transaction_request::{ TransactionRequest, TransactionTemplate },
    },
    store::Store,
};
use miden_client::client::Client;
use crate::client::AzeClient;
use crate::executor::execute_tx_and_sync;
use crate::constants::{ BUY_IN_AMOUNT, TRANSFER_AMOUNT };
use std::rc::Rc;

pub fn create_send_card_note<R: FeltRng, N: NodeRpcClient, S: Store>(
    client: &mut Client<N, R, S>,
    sender_account_id: AccountId,
    target_account_id: AccountId,
    assets: Vec<Asset>,
    note_type: NoteType,
    mut rng: RpoRandomCoin,
    cards: [[Felt; 4]; 2]
) -> Result<Note, NoteError> {
    let note_script = include_str!("../../contracts/notes/game/deal.masm");
    // TODO: hide it under feature flag debug (.with_debug_mode(true))
    let script_ast = ProgramAst::parse(note_script).unwrap();
    let note_script = client.compile_note_script(script_ast, vec![]).unwrap();

    let card_1 = cards[0];
    let card_2 = cards[1];

    let mut inputs = [card_1.as_slice(), card_2.as_slice()].concat();
    println!("card Inputs: {:?}", inputs);

    let note_inputs = NoteInputs::new(inputs).unwrap();
    let tag = NoteTag::from_account_id(target_account_id, NoteExecutionMode::Local)?;
    let serial_num = rng.draw_word();
    let aux = ZERO;

    // TODO: For now hardcoding notes to be public, + Also find out what encrypted notes means
    let metadata = NoteMetadata::new(sender_account_id, NoteType::Public, tag, aux)?;
    let vault = NoteAssets::new(assets)?;
    let recipient = NoteRecipient::new(serial_num, note_script, note_inputs);

    Ok(Note::new(vault, metadata, recipient))
}

pub fn create_play_raise_note<R: FeltRng, N: NodeRpcClient, S: Store>(
    client: &mut Client<N, R, S>,
    sender_account_id: AccountId,
    target_account_id: AccountId,
    assets: Vec<Asset>,
    note_type: NoteType,
    mut rng: RpoRandomCoin,
    player_bet: u8
) -> Result<Note, NoteError> {
    let note_script = include_str!("../../contracts/notes/game/raise.masm");
    let script_ast = ProgramAst::parse(note_script).unwrap();
    let note_script = client.compile_note_script(script_ast, vec![]).unwrap();

    let inputs = vec![Felt::from(player_bet)];
    let note_inputs = NoteInputs::new(inputs).unwrap();
    let tag = NoteTag::from_account_id(target_account_id, NoteExecutionMode::Local)?;
    let serial_num = rng.draw_word();
    let aux = ZERO;

    let metadata = NoteMetadata::new(sender_account_id, NoteType::Public, tag, aux)?;
    let vault = NoteAssets::new(assets)?;
    let recipient = NoteRecipient::new(serial_num, note_script, note_inputs);

    Ok(Note::new(vault, metadata, recipient))
}

pub fn create_play_call_note<R: FeltRng, N: NodeRpcClient, S: Store>(
    client: &mut Client<N, R, S>,
    sender_account_id: AccountId,
    target_account_id: AccountId,
    assets: Vec<Asset>,
    note_type: NoteType,
    mut rng: RpoRandomCoin
) -> Result<Note, NoteError> {
    let note_script = include_str!("../../contracts/notes/game/call.masm");
    let script_ast = ProgramAst::parse(note_script).unwrap();
    let note_script = client.compile_note_script(script_ast, vec![]).unwrap();

    let note_inputs = NoteInputs::new(vec![]).unwrap();
    let tag = NoteTag::from_account_id(target_account_id, NoteExecutionMode::Local)?;
    let serial_num = rng.draw_word();
    let aux = ZERO;

    let metadata = NoteMetadata::new(sender_account_id, NoteType::Public, tag, aux)?;
    let vault = NoteAssets::new(assets)?;
    let recipient = NoteRecipient::new(serial_num, note_script, note_inputs);

    Ok(Note::new(vault, metadata, recipient))
}

// TODO: remove this function after testing
pub async fn mint_note(
    client: &mut AzeClient,
    basic_account_id: AccountId,
    faucet_account_id: AccountId,
    note_type: NoteType
) -> InputNote {
    let (regular_account, _seed) = client.get_account(basic_account_id).unwrap();

    // Create a Mint Tx for 1000 units of our fungible asset
    let fungible_asset = FungibleAsset::new(faucet_account_id, BUY_IN_AMOUNT).unwrap();
    let tx_template = TransactionTemplate::MintFungibleAsset(
        fungible_asset,
        basic_account_id,
        note_type
    );

    println!("Minting Asset");
    let tx_request = client.build_transaction_request(tx_template).unwrap();
    let _ = execute_tx_and_sync(client, tx_request.clone()).await;

    // Check that note is committed and return it
    println!("Fetching Committed Notes...");
    let note_id = tx_request.expected_output_notes()[0].id();
    let note = client.get_input_note(note_id).unwrap();
    note.try_into().unwrap()
}
// TODO: remove it after testing the flow
pub async fn consume_notes(
    client: &mut AzeClient,
    account_id: AccountId,
    input_notes: &[InputNote]
) {
    let tx_template = TransactionTemplate::ConsumeNotes(
        account_id,
        input_notes
            .iter()
            .map(|n| n.id())
            .collect()
    );
    println!("Consuming Note...");
    let tx_request: TransactionRequest = client.build_transaction_request(tx_template).unwrap();
    execute_tx_and_sync(client, tx_request).await;
}
