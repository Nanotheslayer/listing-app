// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

mod g2g_api;
mod config;

use g2g_api::{G2GApiClient, G2GAuthTokens, SkinPrice};
use config::G2GConfig;

// Global API client and config
struct AppState {
    g2g_client: Mutex<G2GApiClient>,
    g2g_config: G2GConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccountFolder {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccountsData {
    pub accounts: Vec<AccountFolder>,
    pub base_path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SkinPriceRequest {
    pub skins: Vec<String>,
    pub server: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SkinPriceResponse {
    pub prices: Vec<SkinPrice>,
    pub total_value: String,
    pub most_expensive: Option<SkinPrice>,
}

#[tauri::command]
async fn load_account_folders(folder_path: String) -> Result<AccountsData, String> {
    println!("Loading account folders from: {}", folder_path);
    
    let path = PathBuf::from(&folder_path);
    
    if !path.exists() {
        return Err("Указанная папка не существует".to_string());
    }
    
    if !path.is_dir() {
        return Err("Указанный путь не является папкой".to_string());
    }
    
    let mut accounts = Vec::new();
    
    match fs::read_dir(&path) {
        Ok(entries) => {
            for entry in entries {
                match entry {
                    Ok(entry) => {
                        let entry_path = entry.path();
                        if entry_path.is_dir() {
                            if let Some(folder_name) = entry_path.file_name() {
                                if let Some(name_str) = folder_name.to_str() {
                                    accounts.push(AccountFolder {
                                        name: name_str.to_string(),
                                        path: entry_path.to_string_lossy().to_string(),
                                    });
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Ошибка чтения записи: {}", e);
                    }
                }
            }
        }
        Err(e) => {
            return Err(format!("Не удалось прочитать содержимое папки: {}", e));
        }
    }
    
    accounts.sort_by(|a, b| a.name.cmp(&b.name));
    
    println!("Найдено {} аккаунтов", accounts.len());
    
    Ok(AccountsData {
        accounts,
        base_path: folder_path,
    })
}

#[tauri::command]
async fn get_account_files(account_path: String) -> Result<Vec<String>, String> {
    println!("Getting files from account: {}", account_path);
    
    let path = PathBuf::from(&account_path);
    
    if !path.exists() || !path.is_dir() {
        return Err("Папка аккаунта не найдена".to_string());
    }
    
    let mut files = Vec::new();
    
    match fs::read_dir(&path) {
        Ok(entries) => {
            for entry in entries {
                if let Ok(entry) = entry {
                    if let Some(file_name) = entry.file_name().to_str() {
                        files.push(file_name.to_string());
                    }
                }
            }
        }
        Err(e) => {
            return Err(format!("Ошибка чтения файлов: {}", e));
        }
    }
    
    Ok(files)
}

#[tauri::command]
async fn read_account_file(account_path: String, file_name: String) -> Result<String, String> {
    println!("Reading file '{}' from account: {}", file_name, account_path);
    
    let account_dir = PathBuf::from(&account_path);
    let file_path = account_dir.join(&file_name);
    
    println!("Full file path: {:?}", file_path);
    
    if !file_path.exists() {
        return Err(format!("Файл не найден: {}", file_name));
    }
    
    match fs::read_to_string(&file_path) {
        Ok(content) => {
            println!("Successfully read {} bytes from {}", content.len(), file_name);
            Ok(content)
        },
        Err(e) => Err(format!("Ошибка чтения файла: {}", e)),
    }
}

#[tauri::command]
async fn read_text_file(path: String) -> Result<String, String> {
    println!("Reading text file: {}", path);
    
    let file_path = PathBuf::from(&path);
    
    if !file_path.exists() {
        return Err(format!("Файл не найден: {}", path));
    }
    
    match fs::read_to_string(&file_path) {
        Ok(content) => Ok(content),
        Err(e) => Err(format!("Ошибка чтения файла: {}", e)),
    }
}

#[tauri::command]
async fn fetch_skin_prices(
    request: SkinPriceRequest,
    state: tauri::State<'_, AppState>
) -> Result<SkinPriceResponse, String> {
    println!("Fetching prices for {} skins on server {}", request.skins.len(), request.server);
    
    let tokens = G2GAuthTokens {
        user_id: state.g2g_config.user_id.clone(),
        refresh_token: state.g2g_config.refresh_token.clone(),
        long_lived_token: state.g2g_config.long_lived_token.clone(),
        active_device_token: state.g2g_config.active_device_token.clone(),
    };
    
    let mut prices = Vec::new();
    let mut total_value = 0.0;
    let mut most_expensive: Option<SkinPrice> = None;
    let mut max_price = 0.0;
    
    for skin in request.skins.iter() {
        println!("Fetching price for: {}", skin);
        
        // Используем tokio::sync::Mutex, поэтому .lock() возвращает Future
        let mut client = state.g2g_client.lock().await;
        let price_result = client.fetch_skin_price(skin, &request.server, &tokens).await;
        drop(client); // Явно освобождаем lock
        
        match price_result {
            Ok(price_str) => {
                let numeric_price = if price_str.starts_with("$") {
                    price_str.get(1..).and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0)
                } else {
                    0.0
                };
                
                if numeric_price > 0.0 {
                    total_value += numeric_price;
                    
                    if numeric_price > max_price {
                        max_price = numeric_price;
                        most_expensive = Some(SkinPrice {
                            skin_name: skin.clone(),
                            price: price_str.clone(),
                        });
                    }
                }
                
                prices.push(SkinPrice {
                    skin_name: skin.clone(),
                    price: price_str,
                });
            }
            Err(e) => {
                println!("Error fetching price for {}: {}", skin, e);
                prices.push(SkinPrice {
                    skin_name: skin.clone(),
                    price: "Error".to_string(),
                });
            }
        }
        
        tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;
    }
    
    Ok(SkinPriceResponse {
        prices,
        total_value: format!("${:.2}", total_value),
        most_expensive,
    })
}

#[tauri::command]
fn get_g2g_config_status(state: tauri::State<'_, AppState>) -> Result<bool, String> {
    state.g2g_config.validate()
        .map(|_| true)
        .map_err(|e| format!("Config validation failed: {}", e))
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! Welcome to G2G App!", name)
}

fn main() {
    let g2g_config = match G2GConfig::from_env() {
        Ok(config) => {
            println!("G2G configuration loaded successfully");
            if let Err(e) = config.validate() {
                eprintln!("Warning: G2G config validation failed: {}", e);
            }
            config
        }
        Err(e) => {
            eprintln!("Error loading G2G configuration: {}", e);
            eprintln!("Please create a .env file with G2G tokens");
            eprintln!("See .env.example for reference");
            
            G2GConfig {
                user_id: String::new(),
                refresh_token: String::new(),
                long_lived_token: String::new(),
                active_device_token: String::new(),
            }
        }
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(AppState {
            g2g_client: Mutex::new(G2GApiClient::new()),
            g2g_config,
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            load_account_folders,
            get_account_files,
            read_account_file,
            read_text_file,
            fetch_skin_prices,
            get_g2g_config_status
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
