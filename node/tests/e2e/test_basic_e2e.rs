mod utils;
use aze_lib::client::{
    AzeClient,
    AzeGameMethods,
    AzeAccountTemplate,
};
use aze_lib::constants::{
    SMALL_BLIND_AMOUNT,
    HIGHEST_BET,
    PLAYER_INITIAL_BALANCE,
    CURRENT_PHASE_SLOT,
    CHECK_COUNTER_SLOT
};
use miden_client::{
    client::{
        accounts::{ AccountTemplate, AccountStorageMode },
    },
    errors::ClientError,
};
use miden_crypto::hash::rpo::RpoDigest;
use miden_crypto::FieldElement;
use miden_objects::{
    Felt,
    accounts::Account,
};
use ansi_term::Colour::{ Cyan, Green, Yellow };

#[tokio::test]
async fn test_e2e() {
    let mut client: AzeClient = utils::create_test_client();

    let (game_account, player1_account_id, faucet_account_id, game_slot_data) = utils::setup_accounts(
        &mut client
    );

    let game_account_id = game_account.id();

    let (player2_account, _) = client
        .new_game_account(
            AzeAccountTemplate::PlayerAccount {
                mutable_code: false,
                storage_mode: AccountStorageMode::Local,
            },
            None
        )
        .unwrap();
    
    let (player3_account, _) = client
        .new_game_account(
            AzeAccountTemplate::PlayerAccount {
                mutable_code: false,
                storage_mode: AccountStorageMode::Local,
            },
            None
        )
        .unwrap();

    let (player4_account, _) = client
        .new_game_account(
            AzeAccountTemplate::PlayerAccount {
                mutable_code: false,
                storage_mode: AccountStorageMode::Local,
            },
            None
        )
        .unwrap();

    println!("{}", Green.paint("Accounts created"));
    println!("{}", Yellow.paint(format!("Game account id: {:?}", game_account_id)));
    println!("{}", Yellow.paint(format!("Player 1 account id: {:?}", player1_account_id)));
    println!("{}", Yellow.paint(format!("Player 2 account id: {:?}", player2_account.id())));
    println!("{}", Yellow.paint(format!("Player 3 account id: {:?}", player3_account.id())));
    println!("{}", Yellow.paint(format!("Player 4 account id: {:?}", player4_account.id())));
    
    utils::fund_account(&mut client, game_account_id, faucet_account_id).await;
    utils::fund_account(&mut client, player1_account_id, faucet_account_id).await;
    utils::fund_account(&mut client, player2_account.id(), faucet_account_id).await;
    utils::fund_account(&mut client, player3_account.id(), faucet_account_id).await;
    utils::fund_account(&mut client, player4_account.id(), faucet_account_id).await;

    // Preflop

    // Player 1 --> Small blind bets SMALL_BLIND_AMOUNT
    println!("{}", Yellow.paint("Small blind's turn. Action: Bet"));
    let player1_bet = SMALL_BLIND_AMOUNT;
    utils::bet(&mut client, player1_account_id, game_account_id, faucet_account_id, player1_bet, 1 as u8).await;
    //check player balance
    let game_account = client.get_account(game_account_id).unwrap().0;
    assert_eq!(
        game_account.storage().get_item(68 as u8),
        RpoDigest::new([Felt::from((PLAYER_INITIAL_BALANCE - player1_bet) as u8), Felt::ZERO, Felt::ZERO, Felt::ZERO])
    );
    println!("{}", Green.paint("Small blind betted"));

    // Player 2 --> Big blind bets SMALL_BLIND_AMOUNT * 2
    println!("{}", Yellow.paint("Big blind's turn. Action: Bet"));
    let player2_bet = SMALL_BLIND_AMOUNT * 2;
    utils::bet(&mut client, player2_account.id(), game_account_id, faucet_account_id, player2_bet, 2 as u8).await;
    // check balance
    let game_account = client.get_account(game_account_id).unwrap().0;
    assert_eq!(
        game_account.storage().get_item(81 as u8),
        RpoDigest::new([Felt::from((PLAYER_INITIAL_BALANCE - player2_bet) as u8), Felt::ZERO, Felt::ZERO, Felt::ZERO])
    );
    println!("{}", Green.paint("Big blind betted"));

    println!("{}", Yellow.paint("Distributing cards to everyone..."));
    // Deal cards to players and assert the account status
    utils::deal_card(&mut client, game_account_id, player1_account_id, faucet_account_id, 0).await;
    utils::deal_card(&mut client, game_account_id, player2_account.id(), faucet_account_id, 2).await;
    utils::deal_card(&mut client, game_account_id, player3_account.id(), faucet_account_id, 4).await;
    utils::deal_card(&mut client, game_account_id, player4_account.id(), faucet_account_id, 6).await;
    println!("{}", Green.paint("Cards distributed"));

    // Player 3 --> Call
    println!("{}", Yellow.paint("Player 3's turn. Action: Call"));
    utils::call(&mut client, player3_account.id(), game_account_id, faucet_account_id, 3).await;
    // check balance
    let game_account = client.get_account(game_account_id).unwrap().0;
    assert_eq!(
        game_account.storage().get_item(94 as u8),
        RpoDigest::new([Felt::from(20 as u8), Felt::ZERO, Felt::ZERO, Felt::ZERO])
    );
    println!("{}", Green.paint("Player 3 called"));

    // Player 4 --> Fold
    println!("{}", Yellow.paint("Player 4's turn. Action: Fold"));
    utils::fold(&mut client, player4_account.id(), game_account_id, faucet_account_id, 4).await;
    println!("{}", Green.paint("Player 4 folded"));

    // Player 1 --> Call
    println!("{}", Yellow.paint("Player 1's turn. Action: Call"));
    utils::call(&mut client, player1_account_id, game_account_id, faucet_account_id, 1).await;
    // check balance
    let game_account = client.get_account(game_account_id).unwrap().0;
    assert_eq!(
        game_account.storage().get_item(68 as u8),
        RpoDigest::new([Felt::from(20 as u8), Felt::ZERO, Felt::ZERO, Felt::ZERO])
    );
    println!("{}", Green.paint("Player 1 called"));

    // Player 2 --> Check
    println!("{}", Yellow.paint("Player 2's turn. Action: Check"));
    utils::check(&mut client, player2_account.id(), game_account_id, faucet_account_id, 2).await;
    // check phase
    let game_account = client.get_account(game_account_id).unwrap().0;
    assert_eq!(
        game_account.storage().get_item(CURRENT_PHASE_SLOT),
        RpoDigest::new([Felt::from(1 as u8), Felt::ZERO, Felt::ZERO, Felt::ZERO])
    );
    println!("{}", Green.paint("Player 2 checked"));
    println!("{}", Cyan.paint("---> Flop revealed"));

    // Player 1 --> Check
    println!("{}", Yellow.paint("Player 1's turn. Action: Check"));
    utils::check(&mut client, player1_account_id, game_account_id, faucet_account_id, 1).await;
    // assert check counter
    let game_account = client.get_account(game_account_id).unwrap().0;
    assert_eq!(
        game_account.storage().get_item(CHECK_COUNTER_SLOT),
        RpoDigest::new([Felt::from(1 as u8), Felt::ZERO, Felt::ZERO, Felt::ZERO])
    );
    println!("{}", Green.paint("Player 1 checked"));

    // Player 2 --> Check
    println!("{}", Yellow.paint("Player 2's turn. Action: Check"));
    utils::check(&mut client, player2_account.id(), game_account_id, faucet_account_id, 2).await;
    // assert check counter
    let game_account = client.get_account(game_account_id).unwrap().0;
    assert_eq!(
        game_account.storage().get_item(CHECK_COUNTER_SLOT),
        RpoDigest::new([Felt::from(2 as u8), Felt::ZERO, Felt::ZERO, Felt::ZERO])
    );
    println!("{}", Green.paint("Player 2 checked"));

    // Player 3 --> Check
    println!("{}", Yellow.paint("Player 3's turn. Action: Check"));
    utils::check(&mut client, player3_account.id(), game_account_id, faucet_account_id, 3).await;
    println!("{}", Green.paint("Player 3 checked"));
    println!("{}", Cyan.paint("---> Turn revealed"));
    
    // check phase
    let game_account = client.get_account(game_account_id).unwrap().0;
    assert_eq!(
        game_account.storage().get_item(CURRENT_PHASE_SLOT),
        RpoDigest::new([Felt::from(2 as u8), Felt::ZERO, Felt::ZERO, Felt::ZERO])
    );

    // Player 1 --> Check
    println!("{}", Yellow.paint("Player 1's turn. Action: Check"));
    utils::check(&mut client, player1_account_id, game_account_id, faucet_account_id, 1).await;
    // assert check counter
    let game_account = client.get_account(game_account_id).unwrap().0;
    assert_eq!(
        game_account.storage().get_item(CHECK_COUNTER_SLOT),
        RpoDigest::new([Felt::from(1 as u8), Felt::ZERO, Felt::ZERO, Felt::ZERO])
    );
    println!("{}", Green.paint("Player 1 checked"));

    // Player 2 --> Raise
    println!("{}", Yellow.paint("Player 2's turn. Action: Raise"));
    utils::raise(&mut client, player2_account.id(), game_account_id, faucet_account_id, 3 * SMALL_BLIND_AMOUNT, 2).await;
    // check balance
    let game_account = client.get_account(game_account_id).unwrap().0;
    assert_eq!(
        game_account.storage().get_item(81 as u8),
        RpoDigest::new([Felt::from(5 as u8), Felt::ZERO, Felt::ZERO, Felt::ZERO])
    );
    println!("{}", Green.paint("Player 2 raised"));

    // Player 3 --> Call
    println!("{}", Yellow.paint("Player 3's turn. Action: Call"));
    utils::call(&mut client, player3_account.id(), game_account_id, faucet_account_id, 3).await;
    println!("{}", Green.paint("Player 3 called"));

    // Player 1 --> Call
    println!("{}", Yellow.paint("Player 1's turn. Action: Call"));
    utils::call(&mut client, player1_account_id, game_account_id, faucet_account_id, 1).await;
    println!("{}", Green.paint("Player 1 called"));
    println!("{}", Cyan.paint("---> River revealed"));

    // check phase
    let game_account = client.get_account(game_account_id).unwrap().0;
    assert_eq!(
        game_account.storage().get_item(CURRENT_PHASE_SLOT),
        RpoDigest::new([Felt::from(3 as u8), Felt::ZERO, Felt::ZERO, Felt::ZERO])
    );
    
    // Player 1 --> Check
    println!("{}", Yellow.paint("Player 1's turn. Action: Check"));
    utils::check(&mut client, player1_account_id, game_account_id, faucet_account_id, 1).await;
    // assert check counter
    let game_account = client.get_account(game_account_id).unwrap().0;
    assert_eq!(
        game_account.storage().get_item(CHECK_COUNTER_SLOT),
        RpoDigest::new([Felt::from(1 as u8), Felt::ZERO, Felt::ZERO, Felt::ZERO])
    );
    println!("{}", Green.paint("Player 1 checked"));

    // Player 2 --> Check
    println!("{}", Yellow.paint("Player 2's turn. Action: Check"));
    utils::check(&mut client, player2_account.id(), game_account_id, faucet_account_id, 2).await;
    // assert check counter
    let game_account = client.get_account(game_account_id).unwrap().0;
    assert_eq!(
        game_account.storage().get_item(CHECK_COUNTER_SLOT),
        RpoDigest::new([Felt::from(2 as u8), Felt::ZERO, Felt::ZERO, Felt::ZERO])
    );
    println!("{}", Green.paint("Player 2 checked"));

    // Player 3 --> Check
    println!("{}", Yellow.paint("Player 3's turn. Action: Check"));
    utils::check(&mut client, player3_account.id(), game_account_id, faucet_account_id, 3).await;
    println!("{}", Green.paint("Player 3 checked"));

    // check phase
    let game_account = client.get_account(game_account_id).unwrap().0;
    assert_eq!(
        game_account.storage().get_item(CURRENT_PHASE_SLOT),
        RpoDigest::new([Felt::from(4 as u8), Felt::ZERO, Felt::ZERO, Felt::ZERO])
    );

    println!("{}", Cyan.paint("---> Showdown"));
}