use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct G2GConfig {
    pub user_id: String,
    pub refresh_token: String,
    pub long_lived_token: String,
    pub active_device_token: String,
}

impl G2GConfig {
    pub fn from_env() -> Result<Self, String> {
        // Пытаемся загрузить .env файл
        if let Err(e) = dotenvy::dotenv() {
            eprintln!("Warning: Could not load .env file: {}", e);
            eprintln!("Attempting to read from environment variables...");
        }

        let user_id = env::var("G2G_USER_ID")
            .map_err(|_| "G2G_USER_ID not found in environment".to_string())?;
        
        let refresh_token = env::var("G2G_REFRESH_TOKEN")
            .map_err(|_| "G2G_REFRESH_TOKEN not found in environment".to_string())?;
        
        let long_lived_token = env::var("G2G_LONG_LIVED_TOKEN")
            .map_err(|_| "G2G_LONG_LIVED_TOKEN not found in environment".to_string())?;
        
        let active_device_token = env::var("G2G_ACTIVE_DEVICE_TOKEN")
            .map_err(|_| "G2G_ACTIVE_DEVICE_TOKEN not found in environment".to_string())?;

        Ok(Self {
            user_id,
            refresh_token,
            long_lived_token,
            active_device_token,
        })
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.user_id.is_empty() {
            return Err("User ID is empty".to_string());
        }
        if self.refresh_token.is_empty() {
            return Err("Refresh token is empty".to_string());
        }
        if self.long_lived_token.is_empty() {
            return Err("Long lived token is empty".to_string());
        }
        if self.active_device_token.is_empty() {
            return Err("Active device token is empty".to_string());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_validation() {
        let valid_config = G2GConfig {
            user_id: "123".to_string(),
            refresh_token: "token".to_string(),
            long_lived_token: "token".to_string(),
            active_device_token: "token".to_string(),
        };
        assert!(valid_config.validate().is_ok());

        let invalid_config = G2GConfig {
            user_id: "".to_string(),
            refresh_token: "token".to_string(),
            long_lived_token: "token".to_string(),
            active_device_token: "token".to_string(),
        };
        assert!(invalid_config.validate().is_err());
    }
}
