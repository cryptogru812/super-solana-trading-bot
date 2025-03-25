use std::time::Duration;
use std::{env, sync::Arc};
use std::str::FromStr;

use anyhow::{anyhow, Result};
use jito_json_rpc_client::jsonrpc_client::rpc_client::RpcClient as JitoRpcClient;
use solana_client::rpc_client::RpcClient;
use solana_sdk::system_transaction;
use solana_sdk::transaction::VersionedTransaction;
use solana_sdk::{
  instruction::Instruction,
  signer::{keypair::Keypair, Signer}, transaction::Transaction,
};
use tracing::{error, info};

use crate::jito::{self, get_tip_account, get_tip_value, wait_for_bundle_confirmation};

pub async fn sign_and_send_transaction(
  client: &RpcClient,
  keypair: &Keypair,
  mut instructions: Vec<Instruction>,
  use_jito: bool,
) -> Result<Vec<String>> {
  let unit_limit = get_unit_limit();
  let unit_price = get_unit_price();

  // If not using Jito, manually set computer unit price and limit
  if !use_jito {
    let modify_compute_units =
      solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(
        unit_limit,
      );
    let add_priority_fee =
      solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_price(
        unit_price,
      );
    instructions.insert(0, modify_compute_units);
    instructions.insert(1, add_priority_fee);
  }

  // Send Init transaction
  let recent_blockhash = client.get_latest_blockhash()?;
  let txn = Transaction::new_signed_with_payer(
    &instructions,
    Some(&keypair.pubkey()),
    &vec![&*keypair],
    recent_blockhash,
  );

  if env::var("TX_SIMULATE").ok() == Some("true".to_string()) {
    let simulate_result = client.simulate_transaction(&txn)?;
    if let Some(logs) = simulate_result.value.logs {
      for log in logs {
        info!("{}", log);
      }
    }
    return match simulate_result.value.err {
      Some(err) => Err(anyhow!("{}", err)),
      None => Ok(vec![]),
    };
  }
  
  let mut txs = vec![];
  if use_jito {
    // jito implementation placeholder
    let tip_account = get_tip_account().await?;
    // jito tip, the upper limit is 0.1
    let mut tip = get_tip_value().await?;
    tip = tip.min(0.1);
    let tip_lamports = ui_amount_to_amount(tip, spl_token::native_mint::DECIMALS);
    info!(
      "tip account: {}, tip(sol): {}, lamports: {}",
      tip_account, tip, tip_lamports
    );

    let jito_client = Arc::new(JitoRpcClient::new(format!(
      "{}/api/v1/bundles",
      jito::BLOCK_ENGINE_URL.to_string()
    )));
    // tip tx
    let mut bundle: Vec<VersionedTransaction> = vec![];
    bundle.push(VersionedTransaction::from(txn));
    bundle.push(VersionedTransaction::from(system_transaction::transfer(
      &keypair,
      &tip_account,
      tip_lamports,
      recent_blockhash,
    )));
    let bundle_id = jito_client.send_bundle(&bundle).await?;
    txs = match wait_for_bundle_confirmation(
      move |id: String| {
        let client = Arc::clone(&jito_client);
        async move {
          let response = client.get_bundle_statuses(&[id]).await;
          let statuses = response.inspect_err(|err| {
            error!("Error fetching bundle status: {:?}", err);
          })?;
          Ok(statuses.value)
        }
      },
      bundle_id,
      Duration::from_millis(1000),
      Duration::from_secs(10),
    )
    .await {
      Ok(signatures) => {  
        // Log success information  
        println!("Bundle confirmed successfully with signatures: {:?}", signatures);  
        signatures // Return the signatures  
      },
      Err(err) => {
        // Check if the error is related to slippage
        if err.to_string().contains("Slippage tolerance exceeded") {
          // Return the slippage error along with any transaction signatures we might have.
          error!("Slippage error detected: {}", err);
          return Ok(vec![format!("Slippage Error: {}", err)]); // Or a custom error string.
        } else {
          // If it's a different error, propagate it.
          println!("Bundle confirmation error: {}", err);
          println!();
          return Err(anyhow!("Bundle confirmation error: {}", err));
        }
      }
    };
  } else {
    let sig = common::rpc::send_txn(&client, &txn, true)?;
    info!("signature: {:?}", sig);
    txs.push(sig.to_string());
  }
  println!("Swap Initialized. It will be finished soon!");
  Ok(txs)
}

pub fn get_unit_price() -> u64 {
  env::var("UNIT_PRICE")
    .ok()
    .and_then(|v| u64::from_str(&v).ok())
    .unwrap_or(20_000)
}

pub fn get_unit_limit() -> u32 {
  env::var("UNIT_LIMIT")
    .ok()
    .and_then(|v| u32::from_str(&v).ok())
    .unwrap_or(200_000)
}

pub fn ui_amount_to_amount(ui_amount: f64, decimals: u8) -> u64 {
  (ui_amount * 10_usize.pow(decimals as u32) as f64) as u64
}
