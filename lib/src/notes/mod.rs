use miden_lib::transaction::TransactionKernel;
use miden_objects::{
    accounts::{Account, AccountCode, AccountId, AccountStorage, StorageSlotType},
    assembly::{ModuleAst, ProgramAst},
    assets::{Asset, AssetVault, FungibleAsset},
    crypto::rand::FeltRng,
    crypto::rand::RpoRandomCoin,
    notes::{Note, NoteScript},
    transaction::TransactionArgs,
    Felt, NoteError, Word,
};
use miden_tx::TransactionExecutor;

pub fn create_deal_note<R: FeltRng>(
    sender_account_id: AccountId,
    target_account_id: AccountId,
    assets: Vec<Asset>,
    mut rng: R,
) -> Result<Note, NoteError> {
    let note_script = include_str!("../../contracts/notes/game/deal.masm");
    let note_assembler = TransactionKernel::assembler();
    let script_ast = ProgramAst::parse(note_script).unwrap();
    let (note_script, _) = NoteScript::new(script_ast, &note_assembler)?;

    // for now hardcoding cards here
    let card_1 = [Felt::new(99), Felt::new(99), Felt::new(99), Felt::new(99)];
    let card_2 = [Felt::new(98), Felt::new(98), Felt::new(98), Felt::new(98)];

    // Here you can add the inputs to the note
    let inputs = [card_1.as_slice(), card_2.as_slice()].concat();
    let tag: Felt = target_account_id.into();
    let serial_num = rng.draw_word();

    Note::new(
        note_script,
        &inputs,
        &assets,
        serial_num,
        sender_account_id,
        tag,
    )
}
