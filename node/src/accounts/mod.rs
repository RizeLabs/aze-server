use miden_objects::{
    accounts::{Account, AccountCode, AccountId, AccountStorage, AccountType, StorageSlotType},
    assembly::ModuleAst,
    assets::AssetVault,
    utils::format,
    AccountError, Word, ZERO,
};

use miden_lib::{AuthScheme, transaction::TransactionKernel};

// method to create a basic aze game account
pub fn create_basic_aze_game_account(
    init_seed: [u8; 32],
    auth_scheme: AuthScheme,
    account_type: AccountType,
) -> Result<(Account, Word), AccountError> {
    if matches!(account_type, AccountType::FungibleFaucet | AccountType::NonFungibleFaucet) {
        return Err(AccountError::AccountIdInvalidFieldElement(
            "Basic aze accounts cannot have a faucet account type".to_string(),
        ));
    }

    let (_, storage_slot_0_data): (&str, Word) = match auth_scheme {
        AuthScheme::RpoFalcon512 { pub_key } => ("basic::auth_tx_rpo_falcon512", pub_key.into()),
    };

    let aze_game_account_code_src: &str = include_str!("../../contracts/game.masm");

    let aze_game_account_code_ast = ModuleAst::parse(aze_game_account_code_src)
        .map_err(|e| AccountError::AccountCodeAssemblerError(e.into()))?;
    let account_assembler = TransactionKernel::assembler();
    let aze_game_account_code = AccountCode::new(aze_game_account_code_ast.clone(), &account_assembler)?;

    let aze_game_account_storage = AccountStorage::new(vec![(
        0,
        (StorageSlotType::Value { value_arity: 0 }, storage_slot_0_data),
    )])?;
    let account_vault = AssetVault::new(&[]).expect("error on empty vault");

    let account_seed = AccountId::get_account_seed(
        init_seed,
        account_type,
        false,
        aze_game_account_code.root(),
        aze_game_account_storage.root(),
    )?;
    let account_id = AccountId::new(account_seed, aze_game_account_code.root(), aze_game_account_storage.root())?;
    Ok((
        Account::new(account_id, account_vault, aze_game_account_storage, aze_game_account_code, ZERO),
        account_seed,
    ))
}

// method to create basic aze player account in case the user don't have an existing account
pub fn create_basic_aze_player_account(
    init_seed: [u8; 32],
    auth_scheme: AuthScheme,
    account_type: AccountType,
) -> Result<(Account, Word), AccountError> {
    if matches!(account_type, AccountType::FungibleFaucet | AccountType::NonFungibleFaucet) {
        return Err(AccountError::AccountIdInvalidFieldElement(
            "Basic aze player accounts cannot have a faucet account type".to_string(),
        ));
    }

    let (_, storage_slot_0_data): (&str, Word) = match auth_scheme {
        AuthScheme::RpoFalcon512 { pub_key } => ("basic::auth_tx_rpo_falcon512", pub_key.into()),
    };

    let aze_player_account_code_src: &str = include_str!("../../contracts/player.masm");

    let aze_player_account_code_ast = ModuleAst::parse(aze_player_account_code_src)
        .map_err(|e| AccountError::AccountCodeAssemblerError(e.into()))?;
    let account_assembler = TransactionKernel::assembler();
    let aze_player_account_code = AccountCode::new(aze_player_account_code_ast.clone(), &account_assembler)?;

    let aze_player_account_storage = AccountStorage::new(vec![(
        0,
        (StorageSlotType::Value { value_arity: 0 }, storage_slot_0_data),
    )])?;
    let account_vault = AssetVault::new(&[]).expect("error on empty vault");

    let account_seed = AccountId::get_account_seed(
        init_seed,
        account_type,
        false,
        aze_player_account_code.root(),
        aze_player_account_storage.root(),
    )?;
    let account_id = AccountId::new(account_seed, aze_player_account_code.root(), aze_player_account_storage.root())?;
    Ok((
        Account::new(account_id, account_vault, aze_player_account_storage, aze_player_account_code, ZERO),
        account_seed,
    ))
}