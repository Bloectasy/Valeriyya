use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::fs;
use tokio::io::AsyncReadExt;

#[derive(Deserialize, Serialize)]
struct AuthResponse {
    access_token: String,
    expires_in: u64,
    token_type: String,
}

#[derive(Serialize, Deserialize)]
struct StoredToken {
    access_token: String,
    expiry_unix: u64,
}

#[derive(Debug)]
pub struct TokenManager {
    client: Client,
    access_token: String,
    expiry: Instant,
    token_file: String,
    discord_token: String,
    mongo_token: String,
    youtube_token: String,
}

impl TokenManager {
    pub async fn new(token_file: impl Into<String>) -> anyhow::Result<Self> {
        let client = Client::new();
        let token_file = token_file.into();

        let discord_token = std::env::var("VALERIYYA_DISCORD_TOKEN")
            .map_err(|_| anyhow::anyhow!("VALERIYYA_DISCORD_TOKEN not set"))?;
        let mongo_token = std::env::var("VALERIYYA_MONGODB")
            .map_err(|_| anyhow::anyhow!("VALERIYYA_MONGODB not set"))?;
        let youtube_token = std::env::var("VALERIYYA_YOUTUBE_API_KEY")
            .map_err(|_| anyhow::anyhow!("VALERIYYA_YOUTUBE_API_KEY not set"))?;

        if let Some(token) = Self::load_token_from_file(&token_file).await? {
            let now_unix = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
            let duration_left = token.expiry_unix.saturating_sub(now_unix);
            let expiry = Instant::now() + Duration::from_secs(duration_left);

            Ok(Self {
                client,
                access_token: token.access_token,
                expiry,
                token_file,
                discord_token,
                mongo_token,
                youtube_token,
            })
        } else {
            let auth = Self::authenticate(&client).await?;
            let expiry = Instant::now() + Duration::from_secs(auth.expires_in);

            let manager = Self {
                client,
                access_token: auth.access_token.clone(),
                expiry,
                token_file,
                discord_token,
                mongo_token,
                youtube_token,
            };
            manager.save_token_to_file().await?;
            Ok(manager)
        }
    }

    async fn authenticate(client: &Client) -> anyhow::Result<AuthResponse> {
        let client_id = std::env::var("VALERIYYA_SPOTIFY_ID")
            .map_err(|_| anyhow::anyhow!("VALERIYYA_SPOTIFY_ID not set"))?;
        let client_secret = std::env::var("VALERIYYA_SPOTIFY_SECRET")
            .map_err(|_| anyhow::anyhow!("VALERIYYA_SPOTIFY_SECRET not set"))?;

        let response = client
            .post("https://accounts.spotify.com/api/token")
            .form(&[
                ("grant_type", "client_credentials"),
                ("client_id", &client_id),
                ("client_secret", &client_secret),
            ])
            .send()
            .await?
            .error_for_status()?;

        let auth_response = response.json::<AuthResponse>().await?;
        Ok(auth_response)
    }

    pub async fn refresh_spotify_token(&mut self) -> anyhow::Result<()> {
        let auth = Self::authenticate(&self.client).await?;
        self.access_token = auth.access_token;
        self.expiry = Instant::now() + Duration::from_secs(auth.expires_in);
        self.save_token_to_file().await?;
        Ok(())
    }

    pub async fn get_spotify_token(&mut self) -> anyhow::Result<String> {
        if Instant::now() >= self.expiry {
            self.refresh_spotify_token().await?;
        }
        Ok(self.access_token.clone())
    }

    pub fn get_discord_token(&self) -> String {
        self.discord_token.clone()
    }

    pub fn get_mongo_token(&self) -> String {
        self.mongo_token.clone()
    }

    pub fn get_youtube_token(&self) -> String {
        self.youtube_token.clone()
    }

    async fn save_token_to_file(&self) -> anyhow::Result<()> {
        let now_unix = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let seconds_left = self
            .expiry
            .saturating_duration_since(Instant::now())
            .as_secs();
        let expiry_unix = now_unix + seconds_left;

        let token_data = StoredToken {
            access_token: self.access_token.clone(),
            expiry_unix,
        };

        let json_data = serde_json::to_string(&token_data)?;
        fs::write(&self.token_file, json_data).await?;
        Ok(())
    }

    async fn load_token_from_file(token_file: &str) -> anyhow::Result<Option<StoredToken>> {
        if !Path::new(token_file).exists() {
            return Ok(None);
        }
        let mut file = fs::File::open(token_file).await?;
        let mut json_str = String::new();
        file.read_to_string(&mut json_str).await?;
        let token_data: StoredToken = serde_json::from_str(&json_str)?;

        let now_unix = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        if token_data.expiry_unix > now_unix {
            Ok(Some(token_data))
        } else {
            Ok(None)
        }
    }
}
