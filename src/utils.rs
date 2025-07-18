use std::sync::Arc;

use bson::doc;
use chrono::Utc;
use iso8601_duration::Duration as iso_duration;
use mongodb::{options::ClientOptions, Client, Database};
use poise::{
    serenity_prelude::all::{
        Color, CreateEmbed, CreateEmbedAuthor, CreateEmbedFooter, CreateMessage, EditMessage,
        Member, RoleId, Timestamp,
    },
    CreateReply,
};
use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE};
use serde_json::Value;
use serenity::all::{ChannelId, UserId};
use tokio::time::sleep;

use crate::{
    structs::{
        GuildDb, Reminder, ResponsePlaylistApi, ResponseSearchVideoApi, ResponseVideoApi,
        SearchVideoItem, SpotifyQuery, Video,
    },
    Context, Data, Error,
};

#[macro_export]
macro_rules! import {
    [ $($cmd:ident), * ] => {
      $(
        mod $cmd;
        pub use $cmd::$cmd;
      )*
    }
}

#[macro_export]
macro_rules! regex {
    ($re:literal $(,)?) => {{
        static RE: ::once_cell::sync::OnceCell<::regex::Regex> = ::once_cell::sync::OnceCell::new();
        RE.get_or_init(|| ::regex::Regex::new($re).unwrap())
    }};
}

#[macro_export]
macro_rules! regex_lazy {
    ($re:literal $(,)?) => {
        ::once_cell::sync::Lazy::<::regex::Regex>::new(|| regex::Regex::new($re).unwrap())
    };
}

fn extract_spotify_id(url: &str) -> Option<(String, String)> {
    let re = regex::Regex::new(r"open\.spotify\.com/(track|playlist)/([a-zA-Z0-9]+)").unwrap();
    re.captures(url)
        .map(|caps| (caps[1].to_string(), caps[2].to_string()))
}

pub async fn get_spotify_queries(
    url: &str,
    api_key: &str,
    reqwest: &reqwest::Client,
) -> Option<SpotifyQuery> {
    let (kind, id) = extract_spotify_id(url)?;

    match kind.as_str() {
        "track" => {
            let api_url = format!("https://api.spotify.com/v1/tracks/{}", id);
            let response = reqwest
                .get(api_url)
                .header(AUTHORIZATION, format!("Bearer {}", api_key))
                .header(CONTENT_TYPE, "application/json")
                .header(ACCEPT, "application/json")
                .send()
                .await
                .ok()?
                .json::<Value>()
                .await
                .ok()?;

            let name = response.get("name")?.as_str()?;
            let artist = response.get("artists")?.get(0)?.get("name")?.as_str()?;
            Some(SpotifyQuery::Track(format!("{} {}", name, artist)))
        }
        "playlist" | "album" => {
            let mut queries = Vec::new();
            let mut next_url = Some(format!(
                "https://api.spotify.com/v1/playlists/{}/tracks?limit=100",
                id
            ));

            while let Some(url) = next_url {
                let response = reqwest
                    .get(&url)
                    .header(AUTHORIZATION, format!("Bearer {}", api_key))
                    .header(CONTENT_TYPE, "application/json")
                    .header(ACCEPT, "application/json")
                    .send()
                    .await
                    .ok()?
                    .json::<Value>()
                    .await
                    .ok()?;

                if let Some(items) = response.get("items").and_then(|v| v.as_array()) {
                    for item in items {
                        if let Some(track) = item.get("track") {
                            let name = track.get("name").and_then(|n| n.as_str());
                            let artist = track
                                .get("artists")
                                .and_then(|a| a.get(0))
                                .and_then(|a| a.get("name"))
                                .and_then(|n| n.as_str());
                            if let (Some(name), Some(artist)) = (name, artist) {
                                queries.push(format!("{} {}", name, artist));
                            }
                        }
                    }
                }

                next_url = response
                    .get("next")
                    .and_then(|n| n.as_str())
                    .map(|s| s.to_string());
            }
            Some(SpotifyQuery::Playlist(queries))
        }
        _ => None,
    }
}

async fn search_video(
    query: impl Into<String>,
    api_key: &String,
    reqwest: &reqwest::Client,
) -> SearchVideoItem {
    let url = format!(
        "https://youtube.googleapis.com/youtube/v3/search?part=snippet&order=relevance&type=video&maxResults=1&q={}&key={}", 
        query.into(),
        api_key
    );
    let video = reqwest
        .get(url)
        .send()
        .await
        .expect("Error getting Video search.")
        .json::<ResponseSearchVideoApi>()
        .await
        .expect("Error parsing the Video search JSON.");
    video.items
        .first()
        .expect("Error getting the first Video search.")
        .clone()
}

async fn get_metadata(ctx: Context<'_>, url: impl Into<String>) -> Vec<Video> {
    let url = url.into();
    let reqwest_client = reqwest::Client::new();
    let data = ctx.data();
    let mut token_manager = data.api_token_manager.lock().await;
    let spotify_api_key = token_manager.get_spotify_token().await.unwrap().clone();
    let youtube_api_key = token_manager.get_youtube_token().clone();

    // Check if it's a YouTube playlist
    if let Some(youtube_id) =
        regex!(r"(?:(?:PL|LL|EC|UU|FL|RD|UL|TL|PU|OLAK5uy_)[0-9A-Za-z-_]{10,}|RDMM)")
            .find(&url)
            .map(|u| u.as_str().to_owned())
    {
        let request_playlist_url = format!(
            "https://youtube.googleapis.com/youtube/v3/playlistItems?part=snippet%2CcontentDetails&maxResults=100&playlistId={}&key={}",
            youtube_id,
            youtube_api_key
        );
        let playlist_items = reqwest_client
            .get(request_playlist_url)
            .send()
            .await
            .expect("Error getting Playlist JSON.")
            .json::<ResponsePlaylistApi>()
            .await
            .expect("Error parsing the Playlist JSON.")
            .items;

        let mut video_ids: Vec<String> = Vec::with_capacity(100);
        for item in playlist_items.into_iter() {
            video_ids.push(item.snippet.resource_id.video_id);
        }

        let request_videos_url = format!(
            "https://youtube.googleapis.com/youtube/v3/videos?part=snippet%2CcontentDetails&id={}&key={}",
            video_ids.join(","),
            youtube_api_key
        );

        let video_items = reqwest_client
            .get(request_videos_url)
            .send()
            .await
            .expect("Error getting Videos from the Video Id Vector.")
            .json::<ResponseVideoApi>()
            .await
            .expect("Error parsing the Videos JSON.")
            .items;

        let mut videos: Vec<Video> = Vec::with_capacity(100);
        for item in video_items.into_iter() {
            let duration = iso_duration::parse(&item.content_details.duration)
                .unwrap()
                .to_std()
                .unwrap();
            videos.push(Video {
                id: item.id,
                title: item.snippet.title,
                duration,
            });
        }
        return videos;
    }
    // Check if the url is a Spotify playlist
    else if let Some((kind, spotify_id)) = extract_spotify_id(&url) {
        if kind == "playlist" {
            let playlist_url = format!("https://open.spotify.com/playlist/{}", spotify_id);
            let tracks =
                get_spotify_queries(&playlist_url, &spotify_api_key, &reqwest_client).await;
            let mut videos = Vec::new();
            if let Some(SpotifyQuery::Playlist(queries)) = tracks {
                for query in queries {
                    let video = search_video(&query, &youtube_api_key, &reqwest_client).await;
                    let request_video_url = format!(
                        "https://youtube.googleapis.com/youtube/v3/videos?part=snippet%2CcontentDetails&id={}&key={}",
                        video.id.video_id,
                        youtube_api_key
                    );
                    let item = reqwest_client
                        .get(&request_video_url)
                        .send()
                        .await
                        .expect("Error getting Video JSON")
                        .json::<ResponseVideoApi>()
                        .await
                        .expect("Error parsing the Video JSON.")
                        .items
                        .first()
                        .expect("There is no video from this url.")
                        .clone();
                    let duration = iso_duration::parse(&item.content_details.duration)
                        .unwrap()
                        .to_std()
                        .unwrap();
                    videos.push(Video {
                        id: item.id,
                        title: item.snippet.title,
                        duration,
                    });
                }
            }
            return videos;
        }
    }

    let id = match get_spotify_queries(&url, &spotify_api_key, &reqwest_client).await {
        Some(SpotifyQuery::Track(search_query)) => {
            search_video(&search_query, &youtube_api_key, &reqwest_client)
                .await
                .id
                .video_id
        }
        _ => {
            search_video(&url, &youtube_api_key, &reqwest_client)
                .await
                .id
                .video_id
        }
    };

    let request_video_url = format!(
        "https://youtube.googleapis.com/youtube/v3/videos?part=snippet%2CcontentDetails&id={}&key={}",
        id,
        youtube_api_key
    );

    let item = reqwest_client
        .get(request_video_url)
        .send()
        .await
        .expect("Error getting Video JSON")
        .json::<ResponseVideoApi>()
        .await
        .expect("Error parsing the Video JSON.")
        .items
        .first()
        .expect("There is no video from this url.")
        .clone();
    let duration = iso_duration::parse(&item.content_details.duration)
        .unwrap()
        .to_std()
        .unwrap();
    vec![Video {
        id: item.id,
        title: item.snippet.title,
        duration,
    }]
}

fn string_to_sec(raw_text: impl ToString) -> i64 {
    let re = regex_lazy!(
        r"((?P<years>\d+?)\s??y|year|years)?((?P<months>\d+?)\s??month|months)?((?P<weeks>\d+?)\s??w|week|weeks)?((?P<days>\d+?)\s??d|day|days)?((?P<hours>\d+?\s??)h|hour|hours)?((?P<minutes>\d+?)\s??m|min|minutes)?((?P<seconds>\d+?)\s??s|sec|second|seconds)?"
    );

    let text = raw_text.to_string();

    let captures = if let Some(caps) = re.captures(&text) {
        caps
    } else {
        return 0;
    };

    let mut seconds = 0;
    for name in [
        "years", "months", "weeks", "days", "hours", "minutes", "seconds",
    ] {
        if let Some(time) = captures.name(name) {
            let time: i64 = time.as_str().parse().unwrap();

            seconds += match name {
                "years" => time * 31_557_600,
                "months" => time * 2_592_000,
                "weeks" => time * 604_800,
                "days" => time * 86_400,
                "hours" => time * 3_600,
                "minutes" => time * 60,
                "seconds" => time,
                _ => 0,
            };
        } else {
            continue;
        }
    }
    seconds
}

pub async fn get_guild_member(ctx: Context<'_>) -> Result<Option<Member>, Error> {
    Ok(match ctx.guild_id() {
        Some(guild_id) => Some(
            guild_id
                .member(ctx.serenity_context(), ctx.author().id)
                .await?,
        ),
        None => None,
    })
}

pub async fn member_managable<'a>(ctx: Context<'_>, member: &Member) -> bool {
    let bot_id = ctx.serenity_context().cache.current_user().id;
    {
        let guild = match ctx.guild() {
            Some(g) => g,
            None => return false,
        };

        if member.user.id == guild.owner_id || member.user.id == bot_id {
            return false;
        }
        if bot_id == guild.owner_id {
            return true;
        }
    }

    let guild_id = ctx.guild_id().unwrap();

    let bot_member = match guild_id.member(ctx.serenity_context(), bot_id).await {
        Ok(m) => m,
        Err(_) => return false,
    };

    let highest_me_role = bot_member
        .roles
        .iter()
        .filter_map(|role_id| ctx.guild().unwrap().roles.get(role_id).cloned())
        .max_by_key(|role| role.position)
        .map(|role| role.id)
        .unwrap_or_else(|| guild_id.everyone_role());

    let highest_member_role = member
        .roles
        .iter()
        .filter_map(|role_id| ctx.guild().unwrap().roles.get(role_id).cloned())
        .max_by_key(|role| role.position)
        .map(|role| role.id)
        .unwrap_or_else(|| guild_id.everyone_role());

    compare_role_position(ctx, highest_me_role, highest_member_role) > 0
}

pub fn compare_role_position(ctx: Context<'_>, role1: RoleId, role2: RoleId) -> i64 {
    let guild = ctx.guild().unwrap();

    let r1 = guild.roles.get(&role1);
    let r2 = guild.roles.get(&role2);

    match (r1, r2) {
        (Some(role1), Some(role2)) => {
            if role1.position == role2.position {
                return i64::from(role2.id) - i64::from(role1.id);
            }
            (role1.position - role2.position).into()
        }
        _ => 0,
    }
}

pub async fn reminder_checker(ctx: Arc<serenity::prelude::Context>, database: Database) {
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

pub async fn send_reminder(
    ctx: Arc<serenity::prelude::Context>,
    channel_id: ChannelId,
    user_id: UserId,
    message: &str,
) -> Result<(), serenity::Error> {
    let content = format!("<@{}>, here's your reminder: {}", user_id, message);
    let _ = channel_id
        .widen()
        .send_message(&ctx.http, Valeriyya::msg_reply().content(content))
        .await;
    Ok(())
}

pub async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
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

pub async fn initialize_database(database_token: String) -> Client {
    let database_options = ClientOptions::parse(database_token).await.unwrap();
    let db_client = Client::with_options(database_options).unwrap();

    match db_client
        .database("valeriyya")
        .run_command(doc! {"ping": 1 })
        .await
    {
        Ok(..) => {
            tracing::info!("Successfully pinged the database!");
        }
        Err(..) => {
            tracing::error!("Failed to ping the database!");
        }
    }

    db_client
}

pub const PURPLE_COLOR: Color = Color::from_rgb(82, 66, 100);

pub struct Valeriyya;

impl Valeriyya {
    // * Shortcuts to most Create structures
    pub fn embed<'a>() -> CreateEmbed<'a> {
        CreateEmbed::new()
            .color(PURPLE_COLOR)
            .timestamp(Timestamp::now())
    }

    pub fn msg_reply<'a>() -> CreateMessage<'a> {
        CreateMessage::new()
    }

    pub fn msg_edit<'a>() -> EditMessage<'a> {
        EditMessage::new()
    }

    pub fn reply_default<'a>() -> CreateReply<'a> {
        CreateReply::new().ephemeral(true)
    }

    pub fn reply<'a>(content: impl Into<String>) -> CreateReply<'a> {
        CreateReply::new().content(content.into())
    }

    pub fn reply_author<'a>(content: impl Into<String>) -> CreateEmbedAuthor<'a> {
        CreateEmbedAuthor::new(content.into())
    }

    pub fn reply_footer<'a>(content: impl Into<String>) -> CreateEmbedFooter<'a> {
        CreateEmbedFooter::new(content.into())
    }

    // * Utility functions
    pub fn time_format(time: String) -> String {
        format!("<t:{}:R>", time)
    }

    pub fn ms(raw_text: impl ToString) -> i64 {
        string_to_sec(raw_text)
    }

    pub async fn get_database(
        db: &Database,
        guild_id: u64,
    ) -> Result<GuildDb, mongodb::error::Error> {
        GuildDb::new(db, guild_id.to_string()).await
    }

    pub async fn get_metadata(ctx: Context<'_>, url: impl Into<String>) -> Vec<Video> {
        get_metadata(ctx, url).await
    }
}
