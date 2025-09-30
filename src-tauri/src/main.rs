// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

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

// Команда для чтения подпапок из выбранной директории
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
                        // Проверяем, что это папка
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
    
    // Сортируем по имени
    accounts.sort_by(|a, b| a.name.cmp(&b.name));
    
    println!("Найдено {} аккаунтов", accounts.len());
    
    Ok(AccountsData {
        accounts,
        base_path: folder_path,
    })
}

// Команда для получения содержимого папки аккаунта
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

// Команда для чтения текстового файла из папки аккаунта
// НОВАЯ КОМАНДА - принимает путь к папке и имя файла отдельно
#[tauri::command]
async fn read_account_file(account_path: String, file_name: String) -> Result<String, String> {
    println!("Reading file '{}' from account: {}", file_name, account_path);
    
    // Правильно объединяем путь к папке и имя файла
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

// Старая команда (оставляем для совместимости)
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

// Простая команда приветствия (оставляем для совместимости)
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! Welcome to G2G App!", name)
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            load_account_folders,
            get_account_files,
            read_account_file,  // Новая команда
            read_text_file      // Старая команда
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
