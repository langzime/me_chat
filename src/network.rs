use anyhow::Result;
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

pub struct NetworkClient {
    base_url: String,
}

impl NetworkClient {
    pub fn new(base_url: String) -> Self {
        Self { base_url }
    }

    pub async fn login(&self, username: String, password: String) -> Result<LoginResponse> {
        let client = reqwest::Client::new();
        let request = LoginRequest { username, password };
        
        let response = client
            .post(format!("{}/api/login", self.base_url))
            .json(&request)
            .send()
            .await?;
            
        let response = response.json::<LoginResponse>().await?;
        Ok(response)
    }
} 