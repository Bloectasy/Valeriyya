use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Deserialize, Serialize)]
struct AuthResponse {
    access_token: String,
    expires_in: u64,
    token_type: String,
}

#[derive(Serialize, Deserialize)]
struct StoredToken {
    access_token: String,
    expiry_time: u64,
}

pub struct TokenManager {
    client: Client,
    access_token: String,
    expiry_time: Instant,
}

impl TokenManager {
    pub async fn new() -> Self {
        let client = Client::new();

        // Try loading the token from file
        if let Some(token) = Self::load_token_from_file() {
            let expiry_time = Instant::now() + Duration::from_secs(token.expiry_time);
            return Self {
                client,
                access_token: token.access_token,
                expiry_time,
            };
        }

        // If no stored token, authenticate
        let auth = Self::authenticate(&client).await;
        let expiry_time = Instant::now() + Duration::from_secs(auth.expires_in);

        let token_manager = Self {
            client,
            access_token: auth.access_token.clone(),
            expiry_time,
        };

        // Save token to file
        token_manager.save_token_to_file();
        token_manager
    }

    async fn authenticate(client: &Client) -> AuthResponse {
        let response = client
            .post("https://accounts.spotify.com/api/token")
            .form(&[
                ("grant_type", "client_credentials"),
                (
                    "client_id",
                    &std::env::var("VALERIYYA_SPOTIFY_ID").expect("(SPOTIFY_CLIENT_ID IS NOT PRESENT)"),
                ),
                (
                    "client_secret",
                    &std::env::var("VALERIYYA_SPOTIFY_SECRET").expect("(SPOTIFY_CLIENT_SECRET IS NOT PRESENT)"),
                ),
            ])
            .send()
            .await
            .expect("Failed to get token");

        // Log the raw response
        let body = response.text().await.expect("Failed to read response body");

        let auth_response: AuthResponse =
            serde_json::from_str(&body).expect("Failed to parse token response");

        auth_response
    }

    // Updated refresh_token method, behaves like authenticate.
    pub async fn refresh_token(&mut self) {
        // Authenticate again since there's no refresh token.
        let auth = Self::authenticate(&self.client).await;
        self.access_token = auth.access_token;
        self.expiry_time = Instant::now() + Duration::from_secs(auth.expires_in);

        // Save the new token to file
        self.save_token_to_file();
    }

    pub async fn get_token(&mut self) -> String {
        self.access_token.clone()
    }

    pub async fn get_valid_token(&mut self) -> &str {
        if Instant::now() >= self.expiry_time {
            self.refresh_token().await;
        }
        &self.access_token
    }

    fn save_token_to_file(&self) {
        let expiry_unix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + (self.expiry_time - Instant::now()).as_secs();

        let token_data = StoredToken {
            access_token: self.access_token.clone(),
            expiry_time: expiry_unix,
        };

        let json_data = serde_json::to_string(&token_data).expect("Failed to serialize token");

        fs::write("token.json", json_data).expect("Failed to save token to file");
    }

    fn load_token_from_file() -> Option<StoredToken> {
        let mut file = match fs::File::open("token.json") {
            Ok(file) => file,
            Err(_) => return None,
        };

        let mut json_str = String::new();
        file.read_to_string(&mut json_str).ok()?;

        let token_data: StoredToken = serde_json::from_str(&json_str).ok()?;

        let current_unix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if token_data.expiry_time > current_unix {
            Some(token_data)
        } else {
            None
        }
    }
}
