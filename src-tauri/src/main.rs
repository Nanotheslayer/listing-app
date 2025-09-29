// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]


#[tauri::command]
async fn test_api_call(url: String) -> Result<String, String> {
    println!("Making API call to: {}", url);
    
    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;
    
    Ok(format!("Status: {}\n\nResponse:\n{}", status, body))
}

// Команда для G2G API (пример)
#[tauri::command]
async fn fetch_g2g_items(api_key: String) -> Result<String, String> {
    // Здесь будет твоя логика для работы с G2G API
    // Пока просто возвращаем тестовое сообщение
    Ok(format!("G2G API called with key: {}***", &api_key[..5.min(api_key.len())]))
}

// Простая команда приветствия
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! Welcome to G2G App!", name)
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            test_api_call,
            fetch_g2g_items
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
