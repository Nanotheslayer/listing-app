// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use rand::Rng;
use tauri::Emitter;
use chrono;

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
    pub is_listed: bool,
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

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateOfferRequest {
    pub title: String,
    pub description: String,
    pub price: f64,
    pub server: String,
    pub rank: String,
    pub champions_count: i32,
    pub skins_count: i32,
    pub account_path: String,
    pub account_name: String,
}

#[derive(Clone, serde::Serialize)]
struct PriceProgressPayload {
    current: usize,
    total: usize,
    skin_name: String,
    status: String,
}

fn save_offer_id_to_file(account_path: &str, offer_id: &str) -> Result<(), String> {
    println!("💾 Saving offer_id to file...");

    let path = PathBuf::from(account_path);
    let file_path = path.join(format!("{}.txt", offer_id));

    let content = format!(
        "Offer ID: {}\nCreated: {}\nStatus: Live\n",
        offer_id,
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
    );

    fs::write(&file_path, content)
        .map_err(|e| format!("Failed to save offer_id file: {}", e))?;

    println!("✅ Offer ID saved to: {:?}", file_path);
    Ok(())
}

fn check_if_listed(account_path: &str) -> bool {
    if let Ok(entries) = fs::read_dir(account_path) {
        for entry in entries.flatten() {
            if let Some(filename) = entry.file_name().to_str() {
                // Проверяем, начинается ли имя файла с "G17" и заканчивается на .txt
                if filename.starts_with("G17") && filename.ends_with(".txt") {
                    println!("   ✓ Found offer file: {}", filename);
                    return true;
                }
            }
        }
    }
    false
}

#[tauri::command]
async fn create_g2g_offer(
    request: CreateOfferRequest,
    state: tauri::State<'_, AppState>
) -> Result<String, String> {
    println!("🎯 Creating G2G offer...");
    println!("   Title: {}", request.title);
    println!("   Server: {}", request.server);
    println!("   Account path: {}", request.account_path);

    let tokens = G2GAuthTokens {
        user_id: state.g2g_config.user_id.clone(),
        refresh_token: state.g2g_config.refresh_token.clone(),
        long_lived_token: state.g2g_config.long_lived_token.clone(),
        active_device_token: state.g2g_config.active_device_token.clone(),
    };

    // Читаем файл с данными аккаунта
    let account_file = format!("{}.txt", request.account_name);
    let account_path = PathBuf::from(&request.account_path);
    let file_path = account_path.join(&account_file);

    println!("📄 Reading account data from: {:?}", file_path);

    let raw_content = match fs::read_to_string(&file_path) {
        Ok(content) => content,
        Err(e) => {
            return Err(format!("Failed to read account data file: {}", e));
        }
    };

    println!("✅ Account data loaded, {} bytes", raw_content.len());

    // Извлекаем ссылку на скриншот
    let screenshot_url = extract_screenshot_url(&raw_content);
    if let Some(ref url) = screenshot_url {
        println!("🖼️  Found screenshot URL: {}", url);
    } else {
        println!("⚠️  No screenshot URL found in account file");
    }

    // Парсим в CSV формат
    let csv_data = parse_account_to_csv(&raw_content)
        .map_err(|e| format!("Failed to parse account to CSV: {}", e))?;

    println!("✅ Converted to CSV, {} bytes", csv_data.len());

    let mut client = state.g2g_client.lock().await;

    let offer_id = client.create_full_offer_with_data(
        &request.title,
        &request.description,
        request.price,
        &request.server,
        &request.rank,
        request.champions_count,
        request.skins_count,
        &csv_data,
        screenshot_url.as_deref(),
        &tokens,
    ).await?;

    println!("✅ Offer created with data! ID: {}", offer_id);

    // Сохраняем offer_id в файл в папке аккаунта
    save_offer_id_to_file(&request.account_path, &offer_id)?;

    Ok(offer_id)
}

fn extract_screenshot_url(text: &str) -> Option<String> {
    println!("🔍 Searching for screenshot URL in text...");

    for line in text.lines() {
        let line = line.trim();

        // Ищем строку со "Screenshot URL"
        if line.to_lowercase().contains("screenshot url") {
            println!("   Found line with 'screenshot url': {}", line);

            // Ищем любой URL в этой строке (http или https)
            if let Some(http_pos) = line.find("http") {
                // Берём всё от http до конца строки или до пробела
                let url_part = &line[http_pos..];
                let url = url_part.split_whitespace().next().unwrap_or(url_part);

                println!("✅ Extracted screenshot URL: {}", url);
                return Some(url.to_string());
            }
        }

        // Также проверяем прямые ссылки на изображения
        if line.starts_with("http") &&
           (line.contains("imgur.com") ||
            line.contains("gyazo.com") ||
            line.contains("prnt.sc") ||
            line.contains("i.postimg.cc")) {

            // Берём только URL без лишнего текста
            let url = line.split_whitespace().next().unwrap_or(line);
            println!("✅ Found direct image URL: {}", url);
            return Some(url.to_string());
        }
    }

    println!("⚠️  No screenshot URL found in text");
    None
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
                                    let path_str = entry_path.to_string_lossy().to_string();

                                    // Проверяем статус
                                    let is_listed = check_if_listed(&path_str);

                                    accounts.push(AccountFolder {
                                        name: name_str.to_string(),
                                        path: path_str,
                                        is_listed,  // ← Новое поле
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

    let listed_count = accounts.iter().filter(|a| a.is_listed).count();
    println!("Найдено {} аккаунтов ({} в продаже)", accounts.len(), listed_count);

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
async fn open_account_screenshot(account_path: String) -> Result<(), String> {
    println!("Opening screenshot from: {}", account_path);

    let path = PathBuf::from(&account_path);

    if !path.exists() || !path.is_dir() {
        return Err("Папка аккаунта не найдена".to_string());
    }

    // Ищем первый .png файл
    match fs::read_dir(&path) {
        Ok(entries) => {
            for entry in entries {
                if let Ok(entry) = entry {
                    let entry_path = entry.path();
                    if entry_path.is_file() {
                        if let Some(ext) = entry_path.extension() {
                            if ext.to_str() == Some("png") {
                                // Открываем файл через системное приложение
                                let path_str = entry_path.to_string_lossy().to_string();

                                println!("Found PNG file: {}", path_str);

                                #[cfg(target_os = "windows")]
                                {
                                    std::process::Command::new("cmd")
                                        .args(&["/C", "start", "", &path_str])
                                        .spawn()
                                        .map_err(|e| format!("Failed to open image: {}", e))?;
                                }

                                #[cfg(target_os = "macos")]
                                {
                                    std::process::Command::new("open")
                                        .arg(&path_str)
                                        .spawn()
                                        .map_err(|e| format!("Failed to open image: {}", e))?;
                                }

                                #[cfg(target_os = "linux")]
                                {
                                    std::process::Command::new("xdg-open")
                                        .arg(&path_str)
                                        .spawn()
                                        .map_err(|e| format!("Failed to open image: {}", e))?;
                                }

                                println!("Screenshot opened successfully");
                                return Ok(());
                            }
                        }
                    }
                }
            }
            Err("PNG файлы не найдены в папке аккаунта".to_string())
        }
        Err(e) => Err(format!("Ошибка чтения папки аккаунта: {}", e))
    }
}

#[tauri::command]
async fn fetch_skin_prices(
    request: SkinPriceRequest,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>
) -> Result<SkinPriceResponse, String> {
    let total_skins = request.skins.len();
    println!("Fetching prices for {} skins on server {}", total_skins, request.server);

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

    for (index, skin) in request.skins.iter().enumerate() {
        let current = index + 1;

        let _ = app.emit("price-progress", PriceProgressPayload {
            current,
            total: total_skins,
            skin_name: skin.clone(),
            status: "processing".to_string(),
        });

        println!("Fetching price for: {}", skin);

        let mut client = state.g2g_client.lock().await;
        let price_result = client.fetch_skin_price(skin, &request.server, &tokens).await;
        drop(client);

        match price_result {
            Ok(price_str) => {
                let numeric_price = if price_str.starts_with("$") || price_str.starts_with("~$") {
                    price_str.trim_start_matches('~').trim_start_matches('$')
                        .parse::<f64>().ok().unwrap_or(0.0)
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

                let _ = app.emit("price-progress", PriceProgressPayload {
                    current,
                    total: total_skins,
                    skin_name: skin.clone(),
                    status: "completed".to_string(),
                });
            }
            Err(e) => {
                println!("Error fetching price for {}: {}", skin, e);
                prices.push(SkinPrice {
                    skin_name: skin.clone(),
                    price: "Error".to_string(),
                });

                let _ = app.emit("price-progress", PriceProgressPayload {
                    current,
                    total: total_skins,
                    skin_name: skin.clone(),
                    status: "error".to_string(),
                });
            }
        }

        if current < total_skins {
            let delay_ms = rand::thread_rng().gen_range(2000..3500);
            tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
        }
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

#[tauri::command]
async fn create_listing(
    title: String,
    description: String,
    price: f64,
    server: String,
    rank: String,
    champions_count: i32,
    skins_count: i32,
    personal_info: String,
    account_path: String,
    state: tauri::State<'_, AppState>
) -> Result<String, String> {
    println!("📋 Creating listing - received personal_info:");
    println!("   Length: {} chars", personal_info.len());

    // Извлекаем ссылку на скриншот
    let screenshot_url = extract_screenshot_url(&personal_info);
    if let Some(ref url) = screenshot_url {
        println!("🖼️  Found screenshot URL: {}", url);
    } else {
        println!("⚠️  No screenshot URL found in personal info");
    }

    let csv_data = parse_account_to_csv(&personal_info)
        .map_err(|e| format!("Failed to parse account data: {}", e))?;

    println!("✅ Converted to CSV format");
    println!("   CSV length: {} bytes", csv_data.len());

    let tokens = G2GAuthTokens {
        user_id: state.g2g_config.user_id.clone(),
        refresh_token: state.g2g_config.refresh_token.clone(),
        long_lived_token: state.g2g_config.long_lived_token.clone(),
        active_device_token: state.g2g_config.active_device_token.clone(),
    };

    let mut client = state.g2g_client.lock().await;

    let offer_id = client.create_full_offer_with_data(
        &title,
        &description,
        price,
        &server,
        &rank,
        champions_count,
        skins_count,
        &csv_data,
        screenshot_url.as_deref(),  // ← Передаём скриншот
        &tokens,
    ).await?;

    println!("✅ Offer created! ID: {}", offer_id);

    // Сохраняем offer_id в файл
    save_offer_id_to_file(&account_path, &offer_id)?;

    Ok(offer_id)
}

fn parse_account_to_csv(text: &str) -> Result<String, String> {
    let start_marker = "Hi there,";
    let start_idx = text.find(start_marker)
        .ok_or("Could not find 'Hi there,' in account file")?;

    let end_markers = [
        "Thank you for buying from Accounterra,gl&hf!",
        "Thank you for buying from Accounterra, gl&hf!",
        "gl&hf!",
    ];

    let mut end_idx = None;
    for marker in &end_markers {
        if let Some(idx) = text[start_idx..].find(marker) {
            end_idx = Some(start_idx + idx + marker.len());
            break;
        }
    }

    let description_block = if let Some(end) = end_idx {
        &text[start_idx..end]
    } else {
        if let Some(screenshot_idx) = text[start_idx..].find("Screenshot URL") {
            text[start_idx..start_idx + screenshot_idx].trim()
        } else {
            text[start_idx..].trim()
        }
    };

    let mut login = String::new();
    let mut password = String::new();
    let mut email = String::new();
    let mut email_access = String::new();

    for line in description_block.lines() {
        let line = line.trim();
        if line.starts_with("Login:") {
            login = line.replace("Login:", "").trim().to_string();
        } else if line.starts_with("Password:") {
            password = line.replace("Password:", "").trim().to_string();
        } else if line.starts_with("Email is") {
            email = line.replace("Email is", "").trim().to_string();
        } else if line.contains("[DOT]com/email/") && !line.starts_with("Email is") {
            email_access = line.split_whitespace().next().unwrap_or("").to_string();
        }
    }

    if login.is_empty() || password.is_empty() || email.is_empty() {
        return Err(format!(
            "Missing fields - Login: '{}', Password: '{}', Email: '{}'",
            login, password, email
        ));
    }

    // КРИТИЧЕСКИ ВАЖНО: Правильное форматирование CSV
    // В CSV, кавычки внутри кавычек должны быть удвоены ("" вместо ")
    // И используем реальные переносы строк, а не \n
    let description_csv_safe = description_block
        .replace("\"", "\"\"")  // Кавычки -> двойные кавычки (стандарт CSV)
        .replace("\r\n", "\n")  // Нормализуем переносы строк
        .replace("\r", "\n");   // Нормализуем переносы строк

    // CSV формат с реальными переносами строк в description
    let csv_line = format!(
        "{},{},,,,,,,,{},{},\"{}\"\r\n",
        login,
        password,
        email,
        email_access,
        description_csv_safe
    );

    // Проверка
    let comma_count = csv_line.matches(',').count();
    println!("✅ CSV created with {} commas", comma_count);
    println!("   First 200 chars: {}", &csv_line[..csv_line.len().min(200)]);

    Ok(csv_line)
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
            get_g2g_config_status,
            open_account_screenshot,
            create_g2g_offer,
            create_listing
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
