use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginResponse {
    pub success: bool,
    pub message: String,
    pub token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FriendInfo {
    pub id: i64,
    pub username: String,
}

pub struct NetworkClient {
    base_url: String,
    token: std::sync::Mutex<Option<String>>,
    client: reqwest::Client,
}

impl NetworkClient {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            token: std::sync::Mutex::new(None),
            client: reqwest::Client::new(),
        }
    }

    pub async fn login(&self, username: String, password: String) -> anyhow::Result<LoginResponse> {
        println!("[DEBUG] Attempting login for user: {}", &username);
        let request = LoginRequest { username, password };
        
        let response = self.client
            .post(format!("{}/api/login", self.base_url))
            .json(&request)
            .send()
            .await?;

        let mut response = response.json::<LoginResponse>().await?;
        if response.success {
            if let Some(token) = response.token.take() {
                println!("[DEBUG] Login successful, token received: {}", token);
                *self.token.lock().unwrap() = Some(token);
            } else {
                println!("[DEBUG] Login successful but no token received");
            }
        } else {
            println!("[DEBUG] Login failed: {}", response.message);
        }
        Ok(response)
    }

    pub fn get_token(&self) -> Option<String> {
        self.token.lock().unwrap().clone()
    }

    pub async fn get_friend_list(&self) -> anyhow::Result<Vec<FriendInfo>> {
        let token = self.get_token().unwrap_or_default();
        println!("[DEBUG] Attempting to get friend list with token: {}", token);
        
        let response = self.client
            .get(format!("{}/api/friends", self.base_url))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;
            
        println!("[DEBUG] Friend list response status: {}", response.status());
        let response = response.json::<Vec<FriendInfo>>().await?;
        println!("[DEBUG] Successfully got {} friends", response.len());
        Ok(response)
    }
} 