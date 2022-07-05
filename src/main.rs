//! Simple telegram bot that allows authorized users to open the door by sending a dedicated
//! command. Actual door opening is done by controlling a modem which in turn dials a special
//! number to trigger the door opening mechanism.

use futures::lock::Mutex;
use std::sync::Arc;
use teloxide::{
    prelude::*,
    types::{Update, UserId},
    utils::command::BotCommands,
};

mod config;
mod modem;

#[derive(BotCommands, Clone)]
#[command(rename = "snake_case", description = "Simple commands")]
enum SimpleCommand {
    #[command(description = "shows this message.")]
    Help,
    #[command(description = "shows your Telegram ID.")]
    MyId,
    #[command(description = "opens the door.")]
    OpenDoor,
}

#[derive(BotCommands, Clone)]
#[command(rename = "snake_case", description = "Maintainer commands")]
enum MaintainerCommands {
    #[command(description = "list IDs of authorized users.")]
    ListUsers,
    #[command(parse_with = "split", description = "Add a new authorized user.")]
    AddUser { name: String, id: u64 },
    #[command(description = "Remove an authorized user by their name.")]
    RemoveUser { name: String },
    #[command(description = "Send a AT command to the modem.")]
    SendModemCmd { modem_cmd: String },
}

#[tokio::main]
async fn main() {
    let mut args = std::env::args();
    if args.len() < 4 {
        eprintln!(
            "Usage: {} <config_file_path> <serial_port> <baud_rate>",
            args.next().unwrap()
        );
        std::process::exit(1);
    }

    let config_file_path = args.nth(1).unwrap();
    let tty_path = args.next().unwrap();
    let baud_rate: u32 = args.next().unwrap().parse().unwrap();

    let config_storage = config::ConfigStorage::new(&config_file_path);
    let maintainer_id = config_storage.get_config().maintainer_id;
    // TODO: Make the ConfigStorage type thread safe by itself.
    let config_storage: Arc<Mutex<config::ConfigStorage>> = Arc::new(Mutex::new(config_storage));

    let modem = Arc::new(Mutex::new(modem::Modem::new(&tty_path, baud_rate)));

    pretty_env_logger::init();
    log::info!("Starting door opening bot...");

    let bot = Bot::from_env().auto_send();

    let handler = Update::filter_message()
        .branch(
            dptree::entry()
                .filter_command::<SimpleCommand>()
                .endpoint(simple_commands_handler),
        )
        .branch(
            dptree::filter(|msg: Message, maintainer_id: UserId| {
                msg.from()
                    .map(|user| user.id == maintainer_id)
                    .unwrap_or_default()
            })
            .filter_command::<MaintainerCommands>()
            .endpoint(maintainer_commands_handler),
        );

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![maintainer_id, config_storage, modem])
        .default_handler(|upd| async move {
            log::warn!("Unhandled update: {:?}", upd);
        })
        .error_handler(LoggingErrorHandler::with_custom_text(
            "An error has occurred in the dispatcher",
        ))
        .build()
        .setup_ctrlc_handler()
        .dispatch()
        .await;
}

async fn simple_commands_handler(
    msg: Message,
    bot: AutoSend<Bot>,
    cmd: SimpleCommand,
    maintainer_id: UserId,
    config_storage: Arc<Mutex<config::ConfigStorage>>,
    modem: Arc<Mutex<modem::Modem>>,
) -> Result<(), teloxide::RequestError> {
    let user_id = &msg.from().unwrap().id;
    let text = match cmd {
        SimpleCommand::Help => {
            format!(
                "{}\n\n{}",
                SimpleCommand::descriptions(),
                MaintainerCommands::descriptions()
            )
        }
        SimpleCommand::MyId => {
            format!("{}", user_id)
        }
        SimpleCommand::OpenDoor => {
            let config_storage = config_storage.lock().await;
            let user = &config_storage
                .get_config()
                .authorized_users
                .iter()
                .find(|u| u.id == user_id.0);
            let username = match user {
                Some(m) => m.name.clone(),
                None => match msg.from().unwrap().username.clone() {
                    Some(username) => username,
                    None => msg.from().unwrap().full_name(),
                },
            };

            if user.is_some() || *user_id == maintainer_id {
                log::info!("Door opened by {} ({})", username, user_id);
                bot.send_message(msg.chat.id, "Door is opening, hang on...")
                    .await?;
                match modem.lock().await.send_open_door_cmd().await {
                    Ok(_) => String::from("Door should be open. Welcome! 🏠"),
                    Err(_) => String::from("Something went wrong 😐"),
                }
            } else {
                log::warn!(
                    "Unauthorized door opening attempt from {} ({})",
                    username,
                    user_id
                );
                String::from("You are not authorized to execute this command 😐")
            }
        }
    };

    bot.send_message(msg.chat.id, text).await?;
    Ok(())
}

async fn maintainer_commands_handler(
    msg: Message,
    bot: AutoSend<Bot>,
    cmd: MaintainerCommands,
    config_storage: Arc<Mutex<config::ConfigStorage>>,
    modem: Arc<Mutex<modem::Modem>>,
) -> Result<(), teloxide::RequestError> {
    match cmd {
        MaintainerCommands::ListUsers => {
            let config_storage = config_storage.lock().await;
            let user_ids: Vec<String> = config_storage
                .get_config()
                .authorized_users
                .iter()
                .map(|u| u.to_string())
                .collect();
            bot.send_message(msg.chat.id, format!("[ {} ]", user_ids.join(", ")))
                .await?;
        }
        MaintainerCommands::AddUser { name, id } => {
            let user = config::AuthorizedUser { name, id };
            let mut config_storage = config_storage.lock().await;
            config_storage
                .get_config_mut()
                .authorized_users
                .push(user.clone());
            match config_storage.save().await {
                Ok(_) => {
                    let response_text = format!("Authorized user '{}' has been added.", user);
                    log::info!("{}", response_text);
                    bot.send_message(msg.chat.id, response_text).await?;
                }
                Err(err) => {
                    let response_text = format!(
                        "User '{}' has been added, but an error occurred while saving: {}.",
                        user, err
                    );
                    log::error!("{}", response_text);
                    bot.send_message(msg.chat.id, response_text).await?;
                }
            };
        }
        MaintainerCommands::RemoveUser { name } => {
            let mut config_storage = config_storage.lock().await;
            let user_index = config_storage
                .get_config()
                .authorized_users
                .iter()
                .position(|u| u.name == name);
            if let Some(index) = user_index {
                config_storage
                    .get_config_mut()
                    .authorized_users
                    .remove(index);
                match config_storage.save().await {
                    Ok(_) => {
                        let response_text = format!("Authorized user '{}' has been removed.", name);
                        log::info!("{}", response_text);
                        bot.send_message(msg.chat.id, response_text).await?;
                    }
                    Err(err) => {
                        let response_text = format!(
                            "User '{}' has been removed, but an error occurred while saving: {}.",
                            name, err
                        );
                        log::error!("{}", response_text);
                        bot.send_message(msg.chat.id, response_text).await?;
                    }
                };
            } else {
                bot.send_message(msg.chat.id, format!("User '{}' not found.", name))
                    .await?;
            }
        }
        MaintainerCommands::SendModemCmd { modem_cmd } => {
            modem
                .lock()
                .await
                .send_string(modem_cmd)
                .await
                .map_err(teloxide::RequestError::Io)?;
        }
    };

    Ok(())
}
