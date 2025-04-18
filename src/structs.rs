use std::sync::Arc;

use bson::doc;
use chrono::{DateTime, Utc};
use mongodb::Database;
use poise::{
    async_trait,
    serenity_prelude::{ChannelId, Http},
};
use serde::{Deserialize, Serialize};
use songbird::{Event, EventContext, EventHandler};

use crate::utils::Valeriyya;

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct GuildDbChannels {
    pub logs: Option<String>,
    pub welcome: Option<String>,
    pub starboard: Option<String>,
}
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct GuildDbRoles {
    pub staff: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Reminder {
    pub id: u32,
    pub user: u64,
    pub message: String,
    pub datetime: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub channel: u64,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum ActionTypes {
    Ban,
    Unban,
    Kick,
    Mute,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Case {
    pub id: u32,
    pub action: ActionTypes,
    pub guild_id: String,
    pub staff_id: String,
    pub target_id: String,
    pub date: i64,
    pub reason: String,
    pub reference: Option<u32>,
    pub expiration: Option<i64>,
    pub message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct History {
    pub id: String,
    pub ban: u16,
    pub kick: u16,
    pub mute: u16,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct GuildDb {
    pub gid: String,
    pub cases: Vec<Case>,
    pub cases_number: u32,
    pub history: Vec<History>,
    pub channels: GuildDbChannels,
    pub roles: GuildDbRoles,
    pub reminders: Vec<Reminder>,
    pub reminder_count: u32,
}

impl GuildDb {
    pub async fn new(db: &Database, guild_id: impl Into<String>) -> Result<Self, mongodb::error::Error> {
        let guild_id = guild_id.into();
        let collection = db.collection::<GuildDb>("guild");

        if let Some(guilddb) = collection
            .find_one(doc! { "gid": &guild_id })
            .await?
        {
            return Ok(guilddb);
        }

        let new_guild = Self::default().guild_id(guild_id.clone());
        let insert_result = collection.insert_one(&new_guild).await?;

        if let Some(inserted_guild) = collection
            .find_one(doc! { "_id": insert_result.inserted_id })
            .await?
        {
            Ok(inserted_guild)
        } else {
            Err(mongodb::error::Error::from(
                mongodb::error::Error::from(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Failed to retrieve inserted document",
                )),
            ))
        }
    }

    #[inline(always)]
    pub fn guild_id(mut self, gid: impl Into<String>) -> Self {
        self.gid = gid.into();
        self
    }

    #[inline(always)]
    pub fn add_cases(mut self, case: Case) -> Self {
        let cases_number = self.cases_number + 1;
        self = self.set_cases_count(cases_number);
        self.cases.push(case);
        self
    }

    #[inline(always)]
    pub fn add_reminder(mut self, reminder: Reminder) -> Self {
        self.reminders.push(reminder);
        self
    }

    #[inline(always)]
    pub fn remove_reminder(mut self, id: u32) -> Self {
        self.reminders.retain(|reminder| reminder.id != id);
        self
    }

    #[inline(always)]
    #[allow(unused)]
    pub fn set_cases(mut self, cases: Vec<Case>) -> Self {
        self.cases = cases;
        self
    }

    #[inline(always)]
    pub fn delete_cases(mut self, index: usize) -> Self {
        self.cases.remove(index);
        self
    }

    #[inline(always)]
    pub fn set_cases_count(mut self, cases_number: u32) -> Self {
        self.cases_number = cases_number;
        self
    }

    #[inline(always)]
    #[allow(unused)]
    pub fn set_history(mut self, history: Vec<History>) -> Self {
        self.history = history;
        self
    }

    #[inline(always)]
    pub fn set_channels(mut self, channels: GuildDbChannels) -> Self {
        self.channels = channels;
        self
    }

    #[inline(always)]
    pub fn set_roles(mut self, roles: GuildDbRoles) -> Self {
        self.roles = roles;
        self
    }

    #[inline(always)]
    pub fn update_case(
        mut self,
        case_id: u32,
        action: CaseUpdateAction,
        value: CaseUpdateValue,
    ) -> Self {
        let c = self.cases.iter_mut().find(|c| c.id == case_id).unwrap();

        match action {
            CaseUpdateAction::Reason => {
                c.reason = value.reason.unwrap();
            }
            CaseUpdateAction::Reference => {
                c.reference = Some(value.reference.unwrap());
            }
        };

        self
    }

    #[inline(always)]
    pub fn get_reminders_for_user(&self, user_id: u64) -> Vec<Reminder> {
        self.reminders
            .iter()
            .filter(|reminder| reminder.user == user_id) 
            .cloned() 
            .collect()
    }

    #[inline(always)]
    pub fn get_reminder_by_id(&self, id: u32) -> Option<Reminder> {
        self.reminders.iter().find(|reminder| reminder.id == id).cloned()
    }

    #[inline(always)]
    pub fn get_due_reminders(&self, now: chrono::DateTime<Utc>) -> Vec<Reminder> {
        self.reminders
            .iter()
            .filter(|reminder| reminder.datetime <= now) 
            .cloned() 
            .collect() 
    }

    pub async fn execute(self, database: &Database) -> Self {
        let db = database.collection::<GuildDb>("guild");
        db.find_one_and_update(
            doc! { "gid": self.gid.clone() },
            doc! {
                "$set": bson::to_document(&self).unwrap()
            }
        )
        .await
        .unwrap()
        .unwrap()
    }
}

impl GuildDbChannels {
    #[inline(always)]
    pub fn set_logs_channel(mut self, logs: Option<String>) -> Self {
        self.logs = logs;
        self
    }

    #[inline(always)]
    pub fn set_welcome_channel(mut self, welcome: Option<String>) -> Self {
        self.welcome = welcome;
        self
    }
    
    #[inline(always)]
    pub fn set_starboard_channel(mut self, starboard: Option<String>) -> Self {
        self.starboard = starboard;
        self
    }

}

impl GuildDbRoles {
    #[inline(always)]
    pub fn set_staff_role(mut self, staff: Option<String>) -> Self {
        self.staff = staff;
        self
    }
}

pub enum CaseUpdateAction {
    Reason,
    Reference,
}

pub struct CaseUpdateValue {
    pub reason: Option<String>,
    pub reference: Option<u32>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ResponseSearchVideoApi {
    pub items: Vec<SearchVideoItem>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SearchVideoItem {
    pub id: SearchVideoId,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SearchVideoId {
    #[serde(rename = "videoId")]
    pub video_id: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct VideoItem {
    pub id: String,
    pub snippet: VideoSnippet,
    #[serde(rename = "contentDetails")]
    pub content_details: ContentDetails,
}

#[derive(Deserialize, Debug, Clone)]
pub struct VideoSnippet {
    pub title: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ContentDetails {
    pub duration: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ResponseVideoApi {
    pub items: Vec<VideoItem>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PlaylistSnippet {
    #[serde(rename = "resourceId")]
    pub resource_id: ResourceId,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ResourceId {
    #[serde(rename = "videoId")]
    pub video_id: String,
}

#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)]
pub struct PlaylistItem {
    pub id: String,
    pub snippet: PlaylistSnippet,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ResponsePlaylistApi {
    pub items: Vec<PlaylistItem>,
}

#[derive(Clone, Debug)]
pub struct Video {
    pub id: String,
    pub title: String,
    pub duration: std::time::Duration,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ExternalUrls {
    pub spotify: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Artist {
    pub name: String,
    pub external_urls: ExternalUrls,
}

#[derive(Serialize, Deserialize, Debug)]
struct Album {
    pub name: String,
    pub artist: Vec<Artist>,
    pub external_urls: ExternalUrls,
}

pub struct SongEndNotifier {
    pub chan_id: ChannelId,
    pub http: Arc<Http>,
    pub metadata: Video,
}

pub struct SongPlayNotifier {
    pub chan_id: ChannelId,
    pub http: Arc<Http>,
    pub metadata: Video,
}

#[async_trait]
impl EventHandler for SongEndNotifier {
    async fn act(&self, _ctx: &EventContext<'_>) -> Option<Event> {
        let _ = self
            .chan_id
            .send_message(
                &self.http,
                Valeriyya::msg_reply().add_embed(
                    Valeriyya::embed()
                        .description(format!("{} has finished.", self.metadata.title))
                        .title("Song information"),
                ),
            )
            .await;

        None
    }
}

#[async_trait]
impl EventHandler for SongPlayNotifier {
    async fn act(&self, _ctx: &EventContext<'_>) -> Option<Event> {
        let _ = self
            .chan_id
            .send_message(
                &self.http,
                Valeriyya::msg_reply().add_embed(
                    Valeriyya::embed()
                        .description(format!(
                            "Playing [{}]({})",
                            self.metadata.title,
                            format_args!("https://youtu.be/{}", self.metadata.id)
                        ))
                        .title("Song information"),
                ),
            )
            .await;

        None
    }
}
