mod commands;
mod structs;
mod utils;

use mongodb::options::ClientOptions;
use mongodb::{Client, Database};
use poise::serenity_prelude::FullEvent;
use poise::serenity_prelude::{self, GatewayIntents};
use dotenv::dotenv;
use serenity::all::Token;
use ::spotify_auth::TokenManager;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

#[derive(Debug)]
pub struct Data {
    db_client: Client,
    youtube_api_key: String,
    spotify_id: String,
    spotify_secret: String,
    songbird: std::sync::Arc<songbird::Songbird>,
}

impl Data {
    pub fn database(&self) -> Database {
        self.db_client.database("Valeriyya")
    }
}

fn event_listeners(
    event: &FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
) -> Result<(), Error> {
    #[allow(clippy::single_match)]
    match event {
        FullEvent::Ready { data_about_bot } => {
            println!("{} is connected!", data_about_bot.user.name);
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
    let spotify_id = std::env::var("VALERIYYA_SPOTIFY_ID").expect("(SPOTIFY_CLIENT_ID IS NOT PRESENT)");
    let spotify_secret = std::env::var("VALERIYYA_SPOTIFY_SECRET").expect("(SPOTIFY_CLIENT_SECRET IS NOT PRESENT)");

    let songbird = songbird::Songbird::serenity();

    let database_options =
        ClientOptions::parse(database_url).await?;
    let db_client = Client::with_options(database_options)?;
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
        ],
        prefix_options: poise::PrefixFrameworkOptions {
            prefix: Some("!".into()),
            ..Default::default()
        },
        event_handler: |framework, event| {
            Box::pin(async move { event_listeners(event, framework) })
        },
        ..Default::default()
    };

    let data = Data {
        db_client,
        youtube_api_key,
        spotify_id,
        spotify_secret,
        songbird: songbird.clone(),
    };

    let framework = poise::Framework::new(options);

    let mut client = serenity_prelude::ClientBuilder::new(discord_token.clone(), discord_intents)
        // .voice_manager_arc(songbird)
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
    
    // spotify testing area
    let spotify_id = std::env::var("VALERIYYA_SPOTIFY_ID").expect("(SPOTIFY_CLIENT_ID IS NOT PRESENT)");
    let spotify_secret = std::env::var("VALERIYYA_SPOTIFY_SECRET").expect("(SPOTIFY_CLIENT_SECRET IS NOT PRESENT)");

    let mut spotify_token = TokenManager::new(spotify_id, spotify_secret).await;
    println!("{:#?}", spotify_token.get_token().await);
    
    
    
    if let Err(e) = init().await {
        eprintln!("{}", e);
        std::process::exit(1);
    }    
}