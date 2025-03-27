use anyhow::{Result};

mod jito;
mod raydium;
mod sign_and_send_transaction;
mod swap;
mod telegram;
mod utils;

use telegram::telegram_bot::*;

#[tokio::main]
async fn main() -> Result<()> {
  // Start the telegram bot
  let _result = start_bot().await;

  let _swap_result = swap::swap().await;

  Ok(())
}
