use crate::client::AzeClient;
use miden_client::{
    client::transactions::transaction_request::TransactionRequest,
    store::{TransactionFilter},
};


pub async fn execute_tx_and_sync(client: &mut AzeClient, tx_request: TransactionRequest) {
    println!("Executing transaction...");
    client.sync_state().await.unwrap();
    let transaction_execution_result = client.new_transaction(tx_request.clone()).unwrap();
    println!("Got execution result");
    let transaction_id = transaction_execution_result.executed_transaction().id();

    println!("Sending transaction to node");
    client.submit_transaction(transaction_execution_result).await.unwrap();
    println!("Transaction sent to node");

    // wait until tx is committed
    loop {
        println!("Syncing State...");
        client.sync_state().await.unwrap();

        // Check if executed transaction got committed by the node
        let uncommited_transactions =
            client.get_transactions(TransactionFilter::Uncomitted).unwrap();
        let is_tx_committed = uncommited_transactions
            .iter()
            .find(|uncommited_tx| uncommited_tx.id == transaction_id)
            .is_none();

        if is_tx_committed {
            break;
        }

        std::thread::sleep(std::time::Duration::new(3, 0));
    }
}