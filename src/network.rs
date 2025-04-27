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
    pub user_id: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FriendInfo {
    pub id: i64,
    pub username: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: ErrorInfo,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorInfo {
    pub code: i32,
    pub reason: String,
    pub description: String,
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
            .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7")
            .header("Accept-Language", "zh-CN,zh;q=0.9")
            .header("Cache-Control", "max-age=0")
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/135.0.0.0 Safari/537.36")
            .header("Upgrade-Insecure-Requests", "1")
            .header("Proxy-Connection", "keep-alive")
            .send()
            .await?;
            
        let status = response.status();
        println!("[DEBUG] Friend list response status: {}", status);
        let response_text = response.text().await?;
        println!("[DEBUG] Friend list response body: {}", response_text);
        
        if status.is_success() {
            let response = serde_json::from_str::<Vec<FriendInfo>>(&response_text)?;
            println!("[DEBUG] Successfully got {} friends", response.len());
            Ok(response)
        } else {
            let error = serde_json::from_str::<ErrorResponse>(&response_text)?;
            Err(anyhow::anyhow!("Server error: {} - {}", error.error.reason, error.error.description))
        }
    }
} 