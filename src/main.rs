mod commands;
mod structs;
mod utils;

use std::str::FromStr;

use bson::doc;
use mongodb::options::ClientOptions;
use mongodb::{Client, Database};
use poise::serenity_prelude::FullEvent;
use poise::serenity_prelude::{self, GatewayIntents};
use dotenv::dotenv;
use serenity::all::{CreateAttachment, CreateWebhook, ExecuteWebhook, ReactionType, Token};
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
            tracing::info!("{} is connected!", data_about_bot.user.name);
        }
        FullEvent::ReactionAdd { add_reaction, .. } => {
            if let ReactionType::Unicode(ref emoji) = add_reaction.emoji {
            if emoji == "⭐" {
                tracing::info!("There was a star react on the message {}", add_reaction.message_id);

                let channel_id = add_reaction.channel_id;

                if let Some(guild_id) = add_reaction.guild_id {
                    let guild_db = Valeriyya::get_database(&framework.serenity_context.data::<Data>().database(), guild_id.get()).await?;

                    

                    if guild_db.channels.starboard.is_some() {
                        
                        let webhook = channel_id
                            .create_webhook(&framework.serenity_context.http, CreateWebhook::new("Starboard"))
                            .await
                            .unwrap();

                        let message = add_reaction.message(&framework.serenity_context.http).await.unwrap();
                        

                        let mut files = Vec::new();

                        for attachment in message.attachments.iter() {
                            let url = attachment.url.clone();
                            let filename = attachment.filename.clone();

                            match CreateAttachment::url(&framework.serenity_context.http, url.as_str(), filename).await {
                                Ok(file) => files.push(file),
                                Err(err) => {
                                    tracing::error!("Failed to create attachment: {:?}", err);
                                }
                            }
                        }


                        webhook.execute(&framework.serenity_context.http, false, 
                            ExecuteWebhook::new()
                                .content(message.content)
                                .add_files(files)
                            )
                            .await.unwrap();
                            
                    };
                }
            }
            }
        },
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