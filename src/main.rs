mod commands;
mod structs;
mod utils;

use std::str::FromStr;
use std::sync::Arc;

use ::token_manager::TokenManager;
use dotenv::dotenv;
use mongodb::{Client, Database};
use poise::serenity_prelude::{self, GatewayIntents};
use serenity::all::Token;
use tokio::sync::Mutex;

use crate::structs::ValeriyyaEventHandler;
use crate::utils::{initialize_database, on_error};

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

#[derive(Debug)]
pub struct Data {
    db_client: Client,
    api_token_manager: Arc<Mutex<TokenManager>>,
    songbird: Arc<songbird::Songbird>,
}

impl Data {
    pub fn database(&self) -> Database {
        self.db_client.database("Valeriyya")
    }
}

async fn init() -> Result<(), Error> {
    tracing_subscriber::fmt().pretty().init();

    let token_manager = Arc::new(Mutex::new(TokenManager::new("token.json").await?));

    let discord_token = {
        let guard = token_manager.lock().await;
        Token::from_str(&guard.get_discord_token()).expect("Incorrect Discord Token!")
    };
    let songbird = songbird::Songbird::serenity();

    let db_client = {
        let guard = token_manager.lock().await;
        initialize_database(guard.get_mongo_token()).await
    };
    let discord_intents = GatewayIntents::non_privileged()
        | GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::MESSAGE_CONTENT;

    let options = poise::FrameworkOptions {
        commands: vec![
            commands::info::help(),
            commands::info::register(),
            commands::music::play(),
            commands::music::skip(),
            commands::music::leave(),
            commands::music::join(),
            commands::music::loop_music(),
            commands::moderation::ban(),
            commands::moderation::kick(),
            commands::moderation::mute(),
            commands::moderation::cases(),
            commands::moderation::reference(),
            commands::moderation::reason(),
            commands::settings::settings(),
            commands::application::star(),
            commands::info::reminder(),
        ],
        prefix_options: poise::PrefixFrameworkOptions {
            prefix: Some("!".into()),
            mention_as_prefix: true,
            ignore_bots: true,
            edit_tracker: Some(std::sync::Arc::new(poise::EditTracker::for_timespan(
                std::time::Duration::from_secs(60),
            ))),
            ..Default::default()
        },
        owners: {
            let mut owners = std::collections::HashSet::new();
            owners.insert(poise::serenity_prelude::UserId::new(206360333881704449));
            owners
        },
        on_error: |error| Box::pin(on_error(error)),
        initialize_owners: true,
        skip_checks_for_owners: true,
        ..Default::default()
    };

    let data = Data {
        db_client,
        api_token_manager: token_manager,
        songbird: songbird.clone(),
    };

    let framework = poise::Framework::new(options);
    let event_handler = ValeriyyaEventHandler {
        database: data.database().into(),
    };

    let mut client = serenity_prelude::ClientBuilder::new(discord_token.clone(), discord_intents)
        .voice_manager::<songbird::Songbird>(songbird)
        .event_handler(event_handler)
        .framework(framework)
        .data(std::sync::Arc::new(data))
        .await
        .unwrap();

    client.start().await.unwrap();

    Ok(())
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    if let Err(e) = init().await {
        tracing::error!("{}", e);
        std::process::exit(1);
    }
}
