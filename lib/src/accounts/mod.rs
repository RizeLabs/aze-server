use miden_objects::{
    accounts::{
        Account, AccountCode, AccountId, AccountStorage, AccountType, SlotItem, StorageSlot, StorageSlotType
    }, assembly::ModuleAst, assets::{Asset, AssetVault}, AccountError, Felt, FieldElement, Word, ZERO
};

use miden_lib::{transaction::TransactionKernel, AuthScheme};

fn construct_game_constructor_storage(auth_scheme: AuthScheme) -> Vec<SlotItem> {
    let mut game_info: Vec<SlotItem> = vec![];
    // generate 52 cards
    let mut cards: Vec<SlotItem> = vec![];
    // let mut player_pub_keys = vec![];
    let small_blind_amt = 5u8;
    let buy_in_amt = 100u8;
    let no_of_players = 4u8;
    let flop_index = no_of_players * 2 + 1;

    let mut slot_index = 1u8;

    let (_, storage_slot_0_data): (&str, Word) = match auth_scheme {
        AuthScheme::RpoFalcon512 { pub_key } => ("basic::auth_tx_rpo_falcon512", pub_key.into()),
    };

    let auth_slot = SlotItem {
        index: slot_index - 1, // 0th slot
        slot: StorageSlot::new_value(storage_slot_0_data),
    };

    for card_suit in 0..4 {
        for card_number in 1..13 {
            let slot_item: SlotItem = SlotItem {
                index: slot_index,
                slot: StorageSlot {
                    slot_type: StorageSlotType::Value { value_arity: 0 },
                    value: [
                        Felt::from(card_suit as u8),
                        Felt::from(card_number as u8),
                        Felt::ZERO,
                        Felt::ZERO,
                    ],
                }
            };

            cards.push(slot_item);
            slot_index += 1;
        }
    }

    let game_stats = vec![
       SlotItem {
            index: slot_index, // storing next_turn here 
            slot: StorageSlot {
                slot_type: StorageSlotType::Value { value_arity: 0 },
                value: [
                    Felt::ZERO,
                    Felt::ZERO,
                    Felt::ZERO,
                    Felt::ZERO,
                ],
            },
       },
         SlotItem {
                index: slot_index + 1, // storing small blind amt here 
                slot: StorageSlot {
                 slot_type: StorageSlotType::Value { value_arity: 0 },
                 value: [
                      Felt::from(small_blind_amt as u8),
                      Felt::ZERO,
                      Felt::ZERO,
                      Felt::ZERO,
                 ],
                },
            },
        SlotItem {
            index: slot_index + 2, // storing big blind amt here 
            slot: StorageSlot {
                slot_type: StorageSlotType::Value { value_arity: 0 },
                value: [
                    Felt::from(small_blind_amt * 2 as u8),
                    Felt::ZERO,
                    Felt::ZERO,
                    Felt::ZERO,
                ],
            },
        },
        SlotItem {
            index: slot_index + 3, // storing buy in amt here 
            slot: StorageSlot {
                slot_type: StorageSlotType::Value { value_arity: 0 },
                value: [
                    Felt::from(buy_in_amt as u8),
                    Felt::ZERO,
                    Felt::ZERO,
                    Felt::ZERO,
                ],
            },
        },
        SlotItem {
            index: slot_index + 4, // storing no of players here 
            slot: StorageSlot {
                slot_type: StorageSlotType::Value { value_arity: 0 },
                value: [
                    Felt::from(no_of_players as u8),
                    Felt::ZERO,
                    Felt::ZERO,
                    Felt::ZERO,
                ],
            },
        },
        SlotItem {
            index: slot_index + 5, // storing flop index here 
            slot: StorageSlot {
                slot_type: StorageSlotType::Value { value_arity: 0 },
                value: [
                    Felt::from(flop_index as u8),
                    Felt::ZERO,
                    Felt::ZERO,
                    Felt::ZERO,
                ],
            },
        }

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
    game_info.push(auth_slot);
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

    // let (_, storage_slot_0_data): (&str, Word) = match auth_scheme {
    //     AuthScheme::RpoFalcon512 { pub_key } => ("basic::auth_tx_rpo_falcon512", pub_key.into()),
    // };

    let aze_game_account_code_src: &str = include_str!("../../contracts/core/game.masm");

    let aze_game_account_code_ast = ModuleAst::parse(aze_game_account_code_src)
        .map_err(|e| AccountError::AccountCodeAssemblerError(e.into()))?;
    let account_assembler = TransactionKernel::assembler();
    let aze_game_account_code =
        AccountCode::new(aze_game_account_code_ast.clone(), &account_assembler)?;

    // TODO: for now let's skip setting game storage
    let game_constructor_item = construct_game_constructor_storage(auth_scheme);

    // initializing game storage with 52 cards
    let aze_game_account_storage = AccountStorage::new(game_constructor_item)?;

    // we need to fund the account with some fungible asset which it could use to rewards players 
    let account_vault = AssetVault::new(&[]).expect("error on empty vault");

    let account_seed = AccountId::get_account_seed(
        init_seed,
        account_type,
        miden_objects::accounts::AccountStorageType::OnChain,
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
    let aze_player_account_storage = AccountStorage::new(vec![SlotItem {
        index: 0,
        slot: StorageSlot {
            slot_type: StorageSlotType::Value { value_arity: 0 },
            value: storage_slot_0_data,
        },
    }
    ])?;
    let account_vault = AssetVault::new(&[]).expect("error on empty vault");

    let account_seed = AccountId::get_account_seed(
        init_seed,
        account_type,
        miden_objects::accounts::AccountStorageType::OnChain,
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
    // let account_storage = AccountStorage::new(vec![
    //     (
    //         0,
    //         (
    //             StorageSlotType::Value { value_arity: 0 },
    //             public_key,
    //         ),
    //     )
    // ])
    // .unwrap();

    let account_storage = AccountStorage::new(vec![
        SlotItem {
            index: 0,
            slot: StorageSlot {
                slot_type: StorageSlotType::Value { value_arity: 0 },
                value: public_key,
            },
        }
    ])
    .unwrap();

    let account_vault = match assets {
        Some(asset) => AssetVault::new(&[asset]).unwrap(),
        None => AssetVault::new(&[]).unwrap(),
    };

    Account::new(account_id, account_vault, account_storage, account_code, Felt::new(1))
}


const fn account_id(account_type: AccountType, storage: AccountStorageType, rest: u64) -> u64 {
    let mut id = 0;

    id ^= (storage as u64) << 62;
    id ^= (account_type as u64) << 60;
    id ^= rest;

    id
}

pub const ON_CHAIN: u64 = 0b00;
pub const OFF_CHAIN: u64 = 0b10;

#[repr(u64)]
pub enum AccountStorageType {
    OnChain = ON_CHAIN,
    OffChain = OFF_CHAIN,
}

#[test]
fn test_create_account_with_custom_account_code() {
    pub const ACCOUNT_ID_SENDER: u64 = account_id(
        AccountType::RegularAccountImmutableCode,
        AccountStorageType::OffChain,
        0b0001_1111,
    );

    let account_id = AccountId::try_from(ACCOUNT_ID_SENDER).unwrap();

    let public_key = [Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)];
    let _account = get_account_with_custom_account_code(account_id, public_key, None);

    //assert_eq!(account.id().root(), account_id.root());
    //assert_eq!(account.code().root(), AccountCode::new(ModuleAst::default(), &TransactionKernel::assembler()).unwrap().root());
    //assert_eq!(account.storage().root(), AccountStorage::new(vec![]).unwrap().root());
    //assert_eq!(account.vault().root(), AssetVault::new(&[]).unwrap().root());
    //assert_eq!(account.balance(), Felt::new(1));
}