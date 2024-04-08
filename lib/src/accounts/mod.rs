use miden_objects::{
    accounts::{
        Account, AccountCode, AccountId, AccountStorage, AccountType, SlotItem, StorageSlotType,
    },
    assets::Asset,
    assembly::ModuleAst,
    assets::AssetVault,
    AccountError, Felt, FieldElement, Word, ZERO,
};

use miden_lib::{transaction::TransactionKernel, AuthScheme};

fn construct_game_constructor_storage() -> Vec<SlotItem> {
    let mut game_info: Vec<SlotItem> = vec![];
    // generate 52 cards
    let mut cards = vec![];
    // let mut player_pub_keys = vec![];
    let small_blind_amt = 5u8;
    let buy_in_amt = 100u8;
    let no_of_players = 4u8;
    let flop_index = no_of_players * 2 + 1;

    let mut slot_index = 1u8;

    for card_suit in 0..4 {
        for card_number in 1..13 {
            cards.push((
                slot_index, 
                (
                    StorageSlotType::Value { value_arity: 0 },
                    [
                        Felt::from(card_suit as u8),
                        Felt::from(card_number as u8),
                        Felt::ZERO,
                        Felt::ZERO,
                    ],
                ),
            ));
            slot_index += 1;
        }
    }

    let game_stats = vec![
        (
            slot_index, // storing next_turn here 
            (
                StorageSlotType::Value { value_arity: 0 },
                [
                    Felt::ZERO, // for now small blind will always be player 0 we will randomize it later
                    Felt::ZERO,
                    Felt::ZERO,
                    Felt::ZERO,
                ],
            ),
        ),
        (
            slot_index + 1, // storing small blind amt here 
            (
                StorageSlotType::Value { value_arity: 0 },
                [
                    Felt::from(small_blind_amt as u8),
                    Felt::ZERO,
                    Felt::ZERO,
                    Felt::ZERO,
                ],
            ),
        ),
        (
            slot_index + 2,
            (
                StorageSlotType::Value { value_arity: 0 },
                [
                    Felt::from(small_blind_amt * 2 as u8), // big blind amt
                    Felt::ZERO,
                    Felt::ZERO,
                    Felt::ZERO,
                ],
            ),
        ),
        (
            slot_index + 3,
            (
                StorageSlotType::Value { value_arity: 0 },
                [
                    Felt::from(buy_in_amt as u8),  // buy in amt
                    Felt::ZERO,
                    Felt::ZERO,
                    Felt::ZERO,
                ],
            ),
        ),
        (
            slot_index + 4,
            (
                StorageSlotType::Value { value_arity: 0 },
                [
                    Felt::from(no_of_players as u8),  // buy in amt
                    Felt::ZERO,
                    Felt::ZERO,
                    Felt::ZERO,
                ],
            ),
        ),
        (
            slot_index + 5,
            (
                StorageSlotType::Value { value_arity: 0 },
                [
                    Felt::from(flop_index as u8),  // index of flop
                    Felt::ZERO,
                    Felt::ZERO,
                    Felt::ZERO,
                ],
            ),
        ),
        // (
        //     slot_index + 6,
        //     (
        //         StorageSlotType::Value { value_arity: 0 },
        //         [
        //             Felt::ONE,  // raiser as by default raiser would be big blind
        //             Felt::ZERO,
        //             Felt::ZERO,
        //             Felt::ZERO,
        //         ],
        //     ),
        // ),
        // (
        //     slot_index + 7, // storing raiser here
        //     (
        //         StorageSlotType::Value { value_arity: 0 },
        //         [
        //             Felt::ONE,  // raiser as by default raiser would be big blind
        //             Felt::ZERO,
        //             Felt::ZERO,
        //             Felt::ZERO,
        //         ],
        //     ),
        // ),
    ];

    // slot_index += 7;

    // for _ in 0..no_of_players {
    //     player_pub_keys.push(
    //         (
    //             slot_index,
    //             (
    //                 StorageSlotType::Value { value_arity: 0 }, // player public key
    //                 [
    //                     Felt::ZERO,
    //                     Felt::ZERO,
    //                     Felt::ZERO,
    //                     Felt::ZERO,
    //                 ],
    //             ),
    //         )
    //     );

    //     slot_index += 8; // since the mid 9 elements would cover the player stats and initially all those values are zero
    // }

    // merghe player_id with card_suit
    game_info.extend(cards);
    game_info.extend(game_stats);
    // game_info.extend(player_pub_keys);
    game_info
}

// method to create a basic aze game account
// it might also would take in cards but for now we are just initializing it with 52 hardcoded cards
pub fn create_basic_aze_game_account(
    init_seed: [u8; 32],
    auth_scheme: AuthScheme,
    account_type: AccountType,
) -> Result<(Account, Word), AccountError> {
    if matches!(
        account_type,
        AccountType::FungibleFaucet | AccountType::NonFungibleFaucet
    ) {
        return Err(AccountError::AccountIdInvalidFieldElement(
            "Basic aze accounts cannot have a faucet account type".to_string(),
        ));
    }

    let (_, storage_slot_0_data): (&str, Word) = match auth_scheme {
        AuthScheme::RpoFalcon512 { pub_key } => ("basic::auth_tx_rpo_falcon512", pub_key.into()),
    };

    let aze_game_account_code_src: &str = include_str!("../../contracts/core/game.masm");

    let aze_game_account_code_ast = ModuleAst::parse(aze_game_account_code_src)
        .map_err(|e| AccountError::AccountCodeAssemblerError(e.into()))?;
    let account_assembler = TransactionKernel::assembler();
    let aze_game_account_code =
        AccountCode::new(aze_game_account_code_ast.clone(), &account_assembler)?;

    let game_constructor_item = construct_game_constructor_storage();

    // initializing game storage with 52 cards
    let aze_game_account_storage = AccountStorage::new(game_constructor_item)?;


    // we need to fund the account with some fungible asset which it could use to rewards players 
    let account_vault = AssetVault::new(&[]).expect("error on empty vault");

    let account_seed = AccountId::get_account_seed(
        init_seed,
        account_type,
        false,
        aze_game_account_code.root(),
        aze_game_account_storage.root(),
    )?;
    let account_id = AccountId::new(
        account_seed,
        aze_game_account_code.root(),
        aze_game_account_storage.root(),
    )?;
    Ok((
        Account::new(
            account_id,
            account_vault,
            aze_game_account_storage,
            aze_game_account_code,
            ZERO,
        ),
        account_seed,
    ))
}

// method to create basic aze player account in case the user don't have an existing account
pub fn create_basic_aze_player_account(
    init_seed: [u8; 32],
    auth_scheme: AuthScheme,
    account_type: AccountType,
) -> Result<(Account, Word), AccountError> {
    if matches!(
        account_type,
        AccountType::FungibleFaucet | AccountType::NonFungibleFaucet
    ) {
        return Err(AccountError::AccountIdInvalidFieldElement(
            "Basic aze player accounts cannot have a faucet account type".to_string(),
        ));
    }

    let (_, storage_slot_0_data): (&str, Word) = match auth_scheme {
        AuthScheme::RpoFalcon512 { pub_key } => ("basic::auth_tx_rpo_falcon512", pub_key.into()),
    };

    let aze_player_account_code_src: &str = include_str!("../../contracts/core/player.masm");

    let aze_player_account_code_ast = ModuleAst::parse(aze_player_account_code_src)
        .map_err(|e| AccountError::AccountCodeAssemblerError(e.into()))?;
    let account_assembler = TransactionKernel::assembler();
    let aze_player_account_code =
        AccountCode::new(aze_player_account_code_ast.clone(), &account_assembler)?;

    let aze_player_account_storage = AccountStorage::new(vec![(
        0,
        (
            StorageSlotType::Value { value_arity: 0 },
            storage_slot_0_data,
        ),
    )])?;
    let account_vault = AssetVault::new(&[]).expect("error on empty vault");

    let account_seed = AccountId::get_account_seed(
        init_seed,
        account_type,
        false,
        aze_player_account_code.root(),
        aze_player_account_storage.root(),
    )?;
    let account_id = AccountId::new(
        account_seed,
        aze_player_account_code.root(),
        aze_player_account_storage.root(),
    )?;
    Ok((
        Account::new(
            account_id,
            account_vault,
            aze_player_account_storage,
            aze_player_account_code,
            ZERO,
        ),
        account_seed,
    ))
}


pub fn get_account_with_custom_account_code(
    account_id: AccountId,
    public_key: Word,
    assets: Option<Asset>,
) -> Account {
    let account_code_src = include_str!("../../contracts/core/player.masm");

    let account_code_ast = ModuleAst::parse(account_code_src).unwrap();
    let account_assembler = TransactionKernel::assembler();

    let account_code = AccountCode::new(account_code_ast.clone(), &account_assembler).unwrap();
    let account_storage = AccountStorage::new(vec![
        (
            0,
            (
                StorageSlotType::Value { value_arity: 0 },
                public_key,
            ),
        )
    ])
    .unwrap();

    let account_vault = match assets {
        Some(asset) => AssetVault::new(&[asset]).unwrap(),
        None => AssetVault::new(&[]).unwrap(),
    };

    Account::new(account_id, account_vault, account_storage, account_code, Felt::new(1))
}