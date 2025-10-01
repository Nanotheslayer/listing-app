use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct G2GSettings {
    pub user_id: String,
    pub refresh_token: String,
    pub long_lived_token: String,
    pub active_device_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub g2g: G2GSettings,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
}

impl G2GSettings {
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

impl AppSettings {
    // Получить путь к файлу настроек
    fn get_settings_path() -> Result<PathBuf, String> {
        // Для Tauri 2 используем домашнюю директорию + .config
        let home_dir = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map_err(|_| "Failed to get home directory")?;

        #[cfg(target_os = "linux")]
        let config_dir = PathBuf::from(home_dir).join(".config").join("g2g-app");

        #[cfg(target_os = "macos")]
        let config_dir = PathBuf::from(home_dir).join("Library").join("Application Support").join("g2g-app");

        #[cfg(target_os = "windows")]
        let config_dir = PathBuf::from(home_dir).join("AppData").join("Roaming").join("g2g-app");

        // Создаем директорию если не существует
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)
                .map_err(|e| format!("Failed to create config directory: {}", e))?;
        }

        Ok(config_dir.join("settings.json"))
    }

    // Загрузить настройки из файла
    pub fn load() -> Result<Self, String> {
        let settings_path = Self::get_settings_path()?;

        if !settings_path.exists() {
            return Err("Settings file not found".to_string());
        }

        let content = fs::read_to_string(&settings_path)
            .map_err(|e| format!("Failed to read settings file: {}", e))?;

        let settings: AppSettings = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse settings: {}", e))?;

        // Валидация G2G настроек
        settings.g2g.validate()?;

        Ok(settings)
    }

    // Сохранить настройки в файл
    pub fn save(&self) -> Result<(), String> {
        // Валидация перед сохранением
        self.g2g.validate()?;

        let settings_path = Self::get_settings_path()?;

        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize settings: {}", e))?;

        fs::write(&settings_path, json)
            .map_err(|e| format!("Failed to write settings file: {}", e))?;

        println!("✅ Settings saved to: {:?}", settings_path);
        Ok(())
    }

    // Удалить файл настроек
    pub fn clear() -> Result<(), String> {
        let settings_path = Self::get_settings_path()?;

        if settings_path.exists() {
            fs::remove_file(&settings_path)
                .map_err(|e| format!("Failed to delete settings file: {}", e))?;
            println!("✅ Settings file deleted");
        }

        Ok(())
    }

    // Проверить наличие настроек
    pub fn exists() -> bool {
        if let Ok(settings_path) = Self::get_settings_path() {
            settings_path.exists()
        } else {
            false
        }
    }
}

// Fallback: попытка загрузить из .env (для разработки)
pub fn load_from_env() -> Option<G2GSettings> {
    use std::env;

    // Пробуем загрузить .env файл
    let _ = dotenvy::dotenv();

    let user_id = env::var("G2G_USER_ID").ok()?;
    let refresh_token = env::var("G2G_REFRESH_TOKEN").ok()?;
    let long_lived_token = env::var("G2G_LONG_LIVED_TOKEN").ok()?;
    let active_device_token = env::var("G2G_ACTIVE_DEVICE_TOKEN").ok()?;

    Some(G2GSettings {
        user_id,
        refresh_token,
        long_lived_token,
        active_device_token,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_g2g_settings_validation() {
        let valid = G2GSettings {
            user_id: "123".to_string(),
            refresh_token: "token".to_string(),
            long_lived_token: "token".to_string(),
            active_device_token: "token".to_string(),
        };
        assert!(valid.validate().is_ok());

        let invalid = G2GSettings {
            user_id: "".to_string(),
            refresh_token: "token".to_string(),
            long_lived_token: "token".to_string(),
            active_device_token: "token".to_string(),
        };
        assert!(invalid.validate().is_err());
    }
}
