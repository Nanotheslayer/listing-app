// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
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
use std::sync::atomic::AtomicUsize;
use config::{AppSettings, G2GSettings};

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

#[derive(Clone, serde::Serialize)]
struct ListingProgressPayload {
    stage: String,
    current: usize,
    total: usize,
    message: String,
}

// Global API client (без g2g_config)
struct AppState {
    g2g_client: Mutex<G2GApiClient>,
    cancel_price_calc: Arc<AtomicBool>,
}

// Функция для загрузки настроек G2G
fn load_g2g_settings() -> Result<G2GSettings, String> {
    match AppSettings::load() {
        Ok(settings) => {
            println!("✅ G2G settings loaded from file");
            Ok(settings.g2g)
        }
        Err(_) => {
            // Fallback на .env
            config::load_from_env()
                .ok_or_else(|| {
                    "G2G токены не настроены. Перейдите в настройки приложения.".to_string()
                })
        }
    }
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
    app: tauri::AppHandle,  // ← Добавили app handle
    state: tauri::State<'_, AppState>
) -> Result<String, String> {
    let total_stages = 5;

    // Этап 1: Чтение данных
    let _ = app.emit("listing-progress", ListingProgressPayload {
        stage: "reading".to_string(),
        current: 1,
        total: total_stages,
        message: "Чтение данных аккаунта...".to_string(),
    });

    println!("🎯 Creating G2G offer...");
    println!("   Title: {}", request.title);
    println!("   Server: {}", request.server);
    println!("   Account path: {}", request.account_path);

    // Загружаем настройки динамически
    let g2g_settings = load_g2g_settings()
        .map_err(|e| format!("Не удалось загрузить настройки G2G: {}", e))?;

    let tokens = G2GAuthTokens {
        user_id: g2g_settings.user_id.clone(),
        refresh_token: g2g_settings.refresh_token.clone(),
        long_lived_token: g2g_settings.long_lived_token.clone(),
        active_device_token: g2g_settings.active_device_token.clone(),
    };

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

    let screenshot_url = extract_screenshot_url(&raw_content);
    if let Some(ref url) = screenshot_url {
        println!("🖼️  Found screenshot URL: {}", url);
    } else {
        println!("⚠️  No screenshot URL found in account file");
    }

    let csv_data = parse_account_to_csv(&raw_content)
        .map_err(|e| format!("Failed to parse account to CSV: {}", e))?;

    println!("✅ Converted to CSV, {} bytes", csv_data.len());

    // Этап 2: Создание оффера
    let _ = app.emit("listing-progress", ListingProgressPayload {
        stage: "creating".to_string(),
        current: 2,
        total: total_stages,
        message: "Создание объявления...".to_string(),
    });

    let mut client = state.g2g_client.lock().await;

    // Создаем пустой оффер
    let offer_id = client.create_offer_id(&tokens).await?;
    println!("✅ Offer ID created: {}", offer_id);

    // Этап 3: Заполнение информации
    let _ = app.emit("listing-progress", ListingProgressPayload {
        stage: "updating".to_string(),
        current: 3,
        total: total_stages,
        message: "Заполнение информации...".to_string(),
    });

    let delay_ms = rand::thread_rng().gen_range(1500..2500);
    tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;

    // Обновляем оффер и получаем relation_id
    let relation_id = client.update_offer(
        &offer_id,
        &request.title,
        &request.description,
        request.price,
        &request.server,
        &request.rank,
        request.champions_count,
        request.skins_count,
        screenshot_url.as_deref(),
        &tokens,
    ).await?;
    println!("✅ Offer updated, relation_id: {}", relation_id);

    // Этап 4: Загрузка данных
    let _ = app.emit("listing-progress", ListingProgressPayload {
        stage: "uploading".to_string(),
        current: 4,
        total: total_stages,
        message: "Загрузка данных аккаунта...".to_string(),
    });

    let delay_ms = rand::thread_rng().gen_range(1500..2500);
    tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;

    // Загружаем данные аккаунта
    client.upload_account_data(&offer_id, &relation_id, &csv_data, &tokens).await?;
    println!("✅ Account data uploaded");

    // Этап 5: Завершение
    let _ = app.emit("listing-progress", ListingProgressPayload {
        stage: "finishing".to_string(),
        current: 5,
        total: total_stages,
        message: "Сохранение...".to_string(),
    });

    println!("✅ Offer created with data! ID: {}", offer_id);

    save_offer_id_to_file(&request.account_path, &offer_id)?;

    Ok(offer_id)
}

fn extract_screenshot_url(text: &str) -> Option<String> {
    println!("🔍 Searching for screenshot URL in text...");

    for line in text.lines() {
        let line = line.trim();

        if line.to_lowercase().contains("screenshot url") {
            println!("   Found line with 'screenshot url': {}", line);

            if let Some(http_pos) = line.find("http") {
                let url_part = &line[http_pos..];
                let url = url_part.split_whitespace().next().unwrap_or(url_part);

                println!("✅ Extracted screenshot URL: {}", url);
                return Some(url.to_string());
            }
        }

        if line.starts_with("http") &&
           (line.contains("imgur.com") ||
            line.contains("gyazo.com") ||
            line.contains("prnt.sc") ||
            line.contains("i.postimg.cc")) {

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
                                    let is_listed = check_if_listed(&path_str);

                                    accounts.push(AccountFolder {
                                        name: name_str.to_string(),
                                        path: path_str,
                                        is_listed,
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

    match fs::read_dir(&path) {
        Ok(entries) => {
            for entry in entries {
                if let Ok(entry) = entry {
                    let entry_path = entry.path();
                    if entry_path.is_file() {
                        if let Some(ext) = entry_path.extension() {
                            if ext.to_str() == Some("png") {
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

// Tunables for the parallel price fetch pipeline.
// Concurrency is intentionally conservative to avoid triggering G2G rate limits.
const PRICE_FETCH_CONCURRENCY: usize = 3;
// Cap total time per single skin including any 401 retry + refresh.
const PRICE_FETCH_PER_SKIN_TIMEOUT_SECS: u64 = 45;
// Minimum delay between individual requests (jitter applied on top).
const PRICE_FETCH_MIN_DELAY_MS: u64 = 400;
const PRICE_FETCH_MAX_DELAY_MS: u64 = 1200;

// Cancellation-aware sleep so a cancel click aborts a pending delay immediately.
async fn cancellable_sleep(duration_ms: u64, cancel_flag: &Arc<AtomicBool>) -> bool {
    let start = std::time::Instant::now();
    let total = std::time::Duration::from_millis(duration_ms);
    loop {
        if cancel_flag.load(Ordering::Relaxed) {
            return true;
        }
        let elapsed = start.elapsed();
        if elapsed >= total {
            return false;
        }
        let remaining = total - elapsed;
        let tick = remaining.min(std::time::Duration::from_millis(100));
        tokio::time::sleep(tick).await;
    }
}

#[tauri::command]
async fn fetch_skin_prices(
    request: SkinPriceRequest,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<SkinPriceResponse, String> {
    let total_skins = request.skins.len();
    println!(
        "Fetching prices for {} skins on server {} (parallel, concurrency={})",
        total_skins, request.server, PRICE_FETCH_CONCURRENCY
    );

    // Reset cancellation flag before starting.
    state.cancel_price_calc.store(false, Ordering::Relaxed);

    if total_skins == 0 {
        return Ok(SkinPriceResponse {
            prices: Vec::new(),
            total_value: "$0.00".to_string(),
            most_expensive: None,
        });
    }

    let g2g_settings = load_g2g_settings()
        .map_err(|e| format!("Не удалось загрузить настройки G2G: {}", e))?;

    let tokens = Arc::new(G2GAuthTokens {
        user_id: g2g_settings.user_id.clone(),
        refresh_token: g2g_settings.refresh_token.clone(),
        long_lived_token: g2g_settings.long_lived_token.clone(),
        active_device_token: g2g_settings.active_device_token.clone(),
    });

    // Snapshot client state + refresh token once upfront so concurrent tasks
    // don't need to hold the top-level Mutex<G2GApiClient>.
    let (http_client, base_url, session_id, initial_token) = {
        let mut client = state.g2g_client.lock().await;
        if client.current_token().is_none() {
            client
                .refresh_token(&tokens)
                .await
                .map_err(|e| format!("Не удалось получить токен G2G: {}", e))?;
        }
        let token = client
            .current_token()
            .ok_or_else(|| "Не удалось получить токен G2G".to_string())?;
        (
            client.http_client(),
            client.base_url_cloned(),
            client.session_id_cloned(),
            token,
        )
    };

    // Shared token updated by whichever task first encounters a 401.
    let shared_token = Arc::new(tokio::sync::RwLock::new(initial_token));
    // Serializes refresh calls so concurrent 401s don't all hit /refresh_access.
    let refresh_mutex = Arc::new(tokio::sync::Mutex::new(()));

    let semaphore = Arc::new(tokio::sync::Semaphore::new(PRICE_FETCH_CONCURRENCY));
    let cancel_flag = state.cancel_price_calc.clone();
    let completed = Arc::new(AtomicUsize::new(0));

    let mut join_set: tokio::task::JoinSet<(usize, String, Result<String, String>)> =
        tokio::task::JoinSet::new();

    for (index, skin) in request.skins.iter().enumerate() {
        let skin = skin.clone();
        let server = request.server.clone();
        let http_client = http_client.clone();
        let base_url = base_url.clone();
        let session_id = session_id.clone();
        let shared_token = shared_token.clone();
        let refresh_mutex = refresh_mutex.clone();
        let tokens = tokens.clone();
        let semaphore = semaphore.clone();
        let cancel_flag = cancel_flag.clone();
        let app_handle = app.clone();
        let completed = completed.clone();

        join_set.spawn(async move {
            // Wait our turn in the concurrency queue.
            let _permit = match semaphore.acquire_owned().await {
                Ok(p) => p,
                Err(e) => {
                    return (index, skin, Err(format!("Semaphore closed: {}", e)));
                }
            };

            if cancel_flag.load(Ordering::Relaxed) {
                return (index, skin, Err("cancelled".to_string()));
            }

            // Small jitter so requests don't fire in perfectly synchronized bursts.
            let delay_ms = rand::thread_rng()
                .gen_range(PRICE_FETCH_MIN_DELAY_MS..PRICE_FETCH_MAX_DELAY_MS);
            if cancellable_sleep(delay_ms, &cancel_flag).await {
                return (index, skin, Err("cancelled".to_string()));
            }

            let done_so_far = completed.load(Ordering::Relaxed);
            let _ = app_handle.emit(
                "price-progress",
                PriceProgressPayload {
                    current: done_so_far + 1,
                    total: total_skins,
                    skin_name: skin.clone(),
                    status: "processing".to_string(),
                },
            );

            // Wrap the whole per-skin flow (including one 401 refresh+retry) in a
            // single timeout so nothing can hang indefinitely.
            let per_skin_result = tokio::time::timeout(
                std::time::Duration::from_secs(PRICE_FETCH_PER_SKIN_TIMEOUT_SECS),
                async {
                    for attempt in 0..2u8 {
                        let token_now = shared_token.read().await.clone();

                        let search_res = g2g_api::search_skin_price_shared(
                            &http_client,
                            &base_url,
                            &session_id,
                            &token_now,
                            &skin,
                            &server,
                        )
                        .await;

                        match search_res {
                            Ok(price) => return Ok::<String, String>(price),
                            Err(g2g_api::SkinSearchError::Unauthorized) if attempt == 0 => {
                                // Only one task at a time should refresh.
                                let _guard = refresh_mutex.lock().await;
                                let current = shared_token.read().await.clone();
                                if current == token_now {
                                    println!(
                                        "⚠️  401 on '{}', refreshing shared token…",
                                        skin
                                    );
                                    let new_token = g2g_api::refresh_access_token_shared(
                                        &http_client,
                                        &base_url,
                                        &session_id,
                                        &tokens,
                                    )
                                    .await
                                    .map_err(|e| format!("Token refresh failed: {}", e))?;
                                    *shared_token.write().await = new_token;
                                }
                                // Loop and retry with the (now refreshed) token.
                                continue;
                            }
                            Err(g2g_api::SkinSearchError::Unauthorized) => {
                                return Err(
                                    "Unauthorized after refresh attempt".to_string()
                                );
                            }
                            Err(g2g_api::SkinSearchError::Other(e)) => {
                                return Err(e);
                            }
                        }
                    }
                    Err("Exhausted retry attempts".to_string())
                },
            )
            .await;

            let price_result: Result<String, String> = match per_skin_result {
                Ok(inner) => inner,
                Err(_) => Err(format!(
                    "Timed out after {}s",
                    PRICE_FETCH_PER_SKIN_TIMEOUT_SECS
                )),
            };

            let done = completed.fetch_add(1, Ordering::Relaxed) + 1;
            let status = if price_result.is_ok() { "completed" } else { "error" };
            let _ = app_handle.emit(
                "price-progress",
                PriceProgressPayload {
                    current: done,
                    total: total_skins,
                    skin_name: skin.clone(),
                    status: status.to_string(),
                },
            );

            (index, skin, price_result)
        });
    }

    // Collect results as they complete; abort in-flight tasks as soon as the
    // user clicks cancel, rather than waiting for the next natural completion.
    let mut raw_results: Vec<(usize, String, Result<String, String>)> =
        Vec::with_capacity(total_skins);
    let mut was_cancelled = false;

    loop {
        if !was_cancelled && cancel_flag.load(Ordering::Relaxed) {
            was_cancelled = true;
            println!("🛑 Cancellation requested — aborting remaining tasks");
            join_set.abort_all();
        }

        // Race between "a task finished" and "cancel flag flipped".
        let next = tokio::select! {
            joined = join_set.join_next() => joined,
            _ = async {
                // Poll the cancel flag a few times per second. Cheap and avoids
                // needing an extra Notify channel wired through every task.
                while !cancel_flag.load(Ordering::Relaxed) {
                    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
                }
            }, if !was_cancelled => {
                // Loop around so the check at the top runs abort_all().
                continue;
            }
        };

        match next {
            Some(Ok(tuple)) => raw_results.push(tuple),
            Some(Err(e)) => {
                // JoinError — either panic or aborted task. Safe to skip.
                if !e.is_cancelled() {
                    println!("⚠️  Task join error: {}", e);
                }
            }
            None => break,
        }
    }

    // Propagate the cancelled state to the shared token on the client so the
    // next command re-refreshes if needed (token may still be valid, but keeping
    // it is fine).
    {
        let mut client = state.g2g_client.lock().await;
        client.set_current_token(Some(shared_token.read().await.clone()));
    }

    if was_cancelled {
        let _ = app.emit(
            "price-progress",
            PriceProgressPayload {
                current: completed.load(Ordering::Relaxed),
                total: total_skins,
                skin_name: "Cancelled".to_string(),
                status: "cancelled".to_string(),
            },
        );
        return Err("Price calculation cancelled by user".to_string());
    }

    // Restore original input order so the output matches what the user sent.
    raw_results.sort_by_key(|(idx, _, _)| *idx);

    let mut prices = Vec::with_capacity(raw_results.len());
    let mut total_value = 0.0;
    let mut most_expensive: Option<SkinPrice> = None;
    let mut max_price = 0.0;

    for (_, skin, result) in raw_results {
        let price_str = match result {
            Ok(p) => p,
            Err(e) => {
                println!("Error fetching price for {}: {}", skin, e);
                "Error".to_string()
            }
        };

        let numeric_price = if price_str.starts_with('$') || price_str.starts_with("~$") {
            price_str
                .trim_start_matches('~')
                .trim_start_matches('$')
                .parse::<f64>()
                .ok()
                .unwrap_or(0.0)
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
            skin_name: skin,
            price: price_str,
        });
    }

    Ok(SkinPriceResponse {
        prices,
        total_value: format!("${:.2}", total_value),
        most_expensive,
    })
}

#[tauri::command]
fn get_g2g_config_status() -> Result<bool, String> {
    match load_g2g_settings() {
        Ok(settings) => {
            settings.validate()
                .map(|_| true)
                .map_err(|e| format!("Настройки невалидны: {}", e))
        }
        Err(_) => {
            Ok(false)
        }
    }
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

    // Загружаем настройки динамически
    let g2g_settings = load_g2g_settings()
        .map_err(|e| format!("Не удалось загрузить настройки G2G: {}", e))?;

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
        user_id: g2g_settings.user_id.clone(),
        refresh_token: g2g_settings.refresh_token.clone(),
        long_lived_token: g2g_settings.long_lived_token.clone(),
        active_device_token: g2g_settings.active_device_token.clone(),
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
        screenshot_url.as_deref(),
        &tokens,
    ).await?;

    println!("✅ Offer created! ID: {}", offer_id);

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

    let description_csv_safe = description_block
        .replace("\"", "\"\"")
        .replace("\r\n", "\n")
        .replace("\r", "\n");

    let csv_line = format!(
        "{},{},,,,,,,,{},{},\"{}\"\r\n",
        login,
        password,
        email,
        email_access,
        description_csv_safe
    );

    let comma_count = csv_line.matches(',').count();
    println!("✅ CSV created with {} commas", comma_count);
    println!("   First 200 chars: {}", &csv_line[..csv_line.len().min(200)]);

    Ok(csv_line)
}

// Команды настроек
#[tauri::command]
async fn load_settings() -> Result<AppSettings, String> {
    println!("📖 Loading settings from file...");

    match AppSettings::load() {
        Ok(settings) => {
            println!("✅ Settings loaded successfully");
            Ok(settings)
        }
        Err(e) => {
            println!("⚠️ Failed to load settings: {}", e);

            if let Some(g2g_settings) = config::load_from_env() {
                println!("✅ Loaded G2G settings from .env (fallback)");
                Ok(AppSettings {
                    g2g: g2g_settings,
                    theme: None,
                })
            } else {
                Err(e)
            }
        }
    }
}

#[tauri::command]
async fn save_settings(settings: AppSettings) -> Result<(), String> {
    println!("💾 Saving settings to file...");
    println!("   User ID: {}", settings.g2g.user_id);

    settings.save()?;

    println!("✅ Settings saved successfully!");
    Ok(())
}

#[tauri::command]
async fn clear_settings() -> Result<(), String> {
    println!("🗑️ Clearing settings...");
    AppSettings::clear()?;
    println!("✅ Settings cleared!");
    Ok(())
}

#[tauri::command]
fn settings_exist() -> bool {
    AppSettings::exists()
}

#[tauri::command]
fn cancel_price_calculation(state: tauri::State<'_, AppState>) -> Result<(), String> {
    println!("🛑 Cancelling price calculation...");
    state.cancel_price_calc.store(true, Ordering::Relaxed);
    Ok(())
}

fn main() {
    // Не загружаем настройки при старте - они будут загружаться динамически

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(AppState {
            g2g_client: Mutex::new(G2GApiClient::new()),
            cancel_price_calc: Arc::new(AtomicBool::new(false)),
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            load_account_folders,
            get_account_files,
            read_account_file,
            read_text_file,
            fetch_skin_prices,
            cancel_price_calculation,
            get_g2g_config_status,
            open_account_screenshot,
            create_g2g_offer,
            create_listing,
            load_settings,
            save_settings,
            clear_settings,
            settings_exist,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
