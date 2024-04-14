use miden_lib::transaction::TransactionKernel;
use miden_objects::{
    accounts::{Account, AccountCode, AccountId, AccountStorage, StorageSlotType}, assembly::{ModuleAst, ProgramAst}, assets::{Asset, AssetVault, FungibleAsset}, crypto::rand::{FeltRng, RpoRandomCoin}, notes::{
        Note, NoteAssets, NoteExecutionMode, NoteInputs, NoteMetadata, NoteRecipient, NoteScript, NoteTag, NoteType
    }, transaction::TransactionArgs, Felt, NoteError, Word, ZERO
};
use miden_tx::TransactionExecutor;

pub fn create_send_card_note<R: FeltRng>(
    sender_account_id: AccountId,
    target_account_id: AccountId,
    assets: Vec<Asset>,
    note_type: NoteType,
    mut rng: R,
    cards: [Felt; 4],
) -> Result<Note, NoteError> {
    let note_script = include_str!("../../contracts/notes/game/deal.masm");
    let note_assembler = TransactionKernel::assembler();
    let script_ast = ProgramAst::parse(note_script).unwrap();
    let (note_script, _) = NoteScript::new(script_ast, &note_assembler)?;

    // for now hardcoding cards here
    // TODO: For now hardcoding cards need to pass it as argument to this function
    let card_1 = [Felt::new(99), Felt::new(99), Felt::new(99), Felt::new(99)];
    let card_2 = [Felt::new(98), Felt::new(98), Felt::new(98), Felt::new(98)];

    // Here you can add the inputs to the note
    let inputs = [card_1.as_slice(), card_2.as_slice()].concat();
    let note_inputs = NoteInputs::new(inputs).unwrap();
    let tag = NoteTag::from_account_id(target_account_id, NoteExecutionMode::Network)?;
    let serial_num = rng.draw_word();
    let aux = ZERO;

    // TODO: For now hardcoding notes to be public, + Also find out what encrypted notes means
    let metadata = NoteMetadata::new(sender_account_id, NoteType::Public, tag, aux)?;
    let vault = NoteAssets::new(assets)?;
    let recipient = NoteRecipient::new(serial_num, note_script, note_inputs);

    Ok(Note::new(
        vault,
        metadata,
        recipient
    ))
}
