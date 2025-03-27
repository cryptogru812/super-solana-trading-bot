use dotenv::dotenv;
use tracing::{info, warn};
use teloxide::{
  prelude::*,
  types::{Me, MessageKind},
  utils::command::BotCommands,
};

#[derive(BotCommands, Clone)]
#[command(
  rename_rule = "lowercase",
  description = "These commands are supported:",
)]
pub enum Command {
  #[command(description = "Display help message.")]
  Help,
  #[command(description = "Start the bot")]
  Start,
}

pub async fn start_bot() -> Result<(), Box<dyn std::error::Error>> {
  dotenv().ok();
  pretty_env_logger::init();
  info!("Starting telegram bot...");

  let bot = Bot::from_env();
  let bot_commands = Command::bot_commands();

  if bot.set_my_commands(bot_commands).await.is_err() {
    warn!("Failed to set bot commands");
  }

  Dispatcher::builder(
    bot,
    dptree::entry().branch(Update::filter_message().endpoint(message_handler)),
  )
  .build()
  .dispatch()
  .await;

  Ok(())
}

async fn message_handler(bot: Bot, msg: Message, _me: Me) -> ResponseResult<()> {
  dotenv().ok();

  if let MessageKind::WebAppData(data) = msg.kind {
    bot.send_message(msg.chat.id, data.web_app_data.data)
      .await?;
  } else if let Some(_text) = msg.text() {
    bot.send_message(msg.chat.id, "Welcome to the bot!").await?;
  }

  Ok(())
}