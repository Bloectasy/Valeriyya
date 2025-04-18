mod commands;
mod structs;
mod utils;

use std::sync::Arc;

use bson::doc;
use chrono::Utc;
use mongodb::options::ClientOptions;
use mongodb::{Client, Database};
use poise::serenity_prelude::FullEvent;
use poise::serenity_prelude::{self, GatewayIntents};
use dotenv::dotenv;
use serenity::all::{ChannelId, Token, UserId};
use structs::Reminder;
use tokio::time::sleep;
use utils::Valeriyya;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

#[derive(Debug)]
pub struct Data {
    db_client: Client,
    youtube_api_key: String,
    songbird: std::sync::Arc<songbird::Songbird>,
}

impl Data {
    pub fn database(&self) -> Database {
        
        self.db_client.database("Valeriyya")
    }
}

async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
    match error {
        poise::FrameworkError::Command { error, ctx, .. } => {
            tracing::error!("Error in command `{}`: {:?}", ctx.command().name, error,);
        }
        error => {
            if let Err(e) = poise::builtins::on_error(error).await {
                tracing::error!("Error while handling error: {}", e)
            }
        }
    }
}

async fn reminder_checker(ctx: Arc<serenity::prelude::Context>, database: Database) {
    loop {
        let now = Utc::now();

        let guilds = ctx.cache.guilds();

        for guild_id in guilds {
            let guild_id_u64 = guild_id.get(); 

            let db = Valeriyya::get_database(&database, guild_id_u64)
                .await
                .expect("Failed to fetch database");

            let due_reminders: Vec<Reminder> = db.get_due_reminders(now);

            for reminder in due_reminders {
                let user_id = UserId::new(reminder.user);
                let channel_id = ChannelId::new(reminder.channel);
                let message = reminder.message.clone();

                let ctx = Arc::clone(&ctx);
                tokio::spawn(async move {
                    if let Err(err) = send_reminder(ctx, channel_id, user_id, &message).await {
                        eprintln!("Failed to send reminder: {}", err);
                    }
                });

                let mut db = Valeriyya::get_database(&database, guild_id_u64)
                    .await
                    .expect("Failed to fetch database");
                db = db.remove_reminder(reminder.id);
                db.execute(&database).await;
            }
        }

        sleep(tokio::time::Duration::from_secs(60)).await;
    }
}



async fn send_reminder(
    ctx: Arc<serenity::prelude::Context>,
    channel_id: ChannelId,
    user_id: UserId,
    message: &str,
) -> Result<(), serenity::Error> {
    let content = format!("<@{}>, here's your reminder: {}", user_id, message);
    channel_id.say(&ctx.http, content).await?;
    Ok(())
}

async fn event_listeners(
    event: &FullEvent,
    framework: poise::FrameworkContext<'_, Data, Error>,
) -> Result<(), Error>
where
    FullEvent: Send + Sync,
{
    #[allow(clippy::single_match)]
    match event {
        FullEvent::Ready { data_about_bot } => {
            let ctx = framework.serenity_context.clone();
            let database = framework.user_data().database();

            tokio::spawn(async move {
                reminder_checker(ctx.into(), database).await;
            });
            tracing::info!("{} is connected!", data_about_bot.user.name);
        }
        _ => {}
    }

    Ok(())
}



async fn init() -> Result<(), Error> {
    tracing_subscriber::fmt().pretty().init();

    let discord_token = Token::from_env("VALERIYYA_DISCORD_TOKEN").expect("(DISCORD_TOKEN IS NOT PRESENT)");
    let database_url = std::env::var("VALERIYYA_MONGODB").expect("(MONGODB_TOKEN IS NOT PRESENT)");
    let youtube_api_key = std::env::var("VALERIYYA_YOUTUBE_API_KEY").expect("(API_TOKEN IS NOT PRESENT)");
   
    let songbird = songbird::Songbird::serenity();

    let database_options =
        ClientOptions::parse(database_url).await?;
    let db_client = Client::with_options(database_options)?;

    match db_client.database("valeriyya").run_command(doc! {"ping": 1 }).await {
        Ok(..) => {
            tracing::info!("Successfully pinged the database!");
        },
        Err(..) => {
            tracing::error!("Failed to ping the database!");
        }
    }

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
            edit_tracker: Some(std::sync::Arc::new(poise::EditTracker::for_timespan(std::time::Duration::from_secs(60)))),
            ..Default::default()
        },
        event_handler: |framework, event| {
            Box::pin(async move { event_listeners(event, framework).await })
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
        youtube_api_key,
        songbird: songbird.clone(),
    };

    let framework = poise::Framework::new(options);

    let mut client = serenity_prelude::ClientBuilder::new(discord_token.clone(), discord_intents)
        .voice_manager::<songbird::Songbird>(songbird)
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