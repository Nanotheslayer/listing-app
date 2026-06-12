use serde::{Deserialize, Serialize};

/// Одна строка, отправляемая в Google-таблицу через веб-хук Apps Script.
#[derive(Debug, Serialize)]
pub struct SheetRow {
    pub username: String,
    pub offer_id: String,
    pub listed_date: String,
    pub folder: String,
    pub status: String,
}

/// Отправляет строку в Google-таблицу через веб-хук Google Apps Script.
///
/// Намеренно best-effort: вызывающая сторона решает, что делать с ошибкой.
/// Запись в таблицу не должна ломать сам процесс выставления оффера.
pub async fn append_row(webhook_url: &str, row: &SheetRow) -> Result<(), String> {
    println!("📊 Sending listing data to Google Sheet webhook...");
    println!("   Username: {}", row.username);
    println!("   Offer ID: {}", row.offer_id);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let response = client
        .post(webhook_url)
        .json(row)
        .send()
        .await
        .map_err(|e| format!("Failed to send request to sheet webhook: {}", e))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .unwrap_or_else(|_| "<unable to read body>".to_string());

    if !status.is_success() {
        return Err(format!("Sheet webhook returned {}: {}", status, body));
    }

    println!("✅ Sheet updated. Webhook response: {}", body);
    Ok(())
}

/// Одна строка таблицы, возвращаемая веб-хуком в режиме ?list=1.
#[derive(Debug, Deserialize)]
pub struct SheetEntry {
    pub username: String,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub offer_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ListResponse {
    ok: bool,
    #[serde(default)]
    rows: Vec<SheetEntry>,
    #[serde(default)]
    error: Option<String>,
}

/// Запрашивает у веб-хука все строки таблицы (GET ?list=1).
pub async fn fetch_rows(webhook_url: &str) -> Result<Vec<SheetEntry>, String> {
    println!("📊 Fetching existing rows from Google Sheet webhook...");

    let separator = if webhook_url.contains('?') { "&" } else { "?" };
    let url = format!("{}{}list=1", webhook_url, separator);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    // Apps Script отвечает редиректом на googleusercontent.com — reqwest
    // следует за ним по умолчанию.
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to request sheet rows: {}", e))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response body: {}", e))?;

    if !status.is_success() {
        return Err(format!("Sheet webhook returned {}: {}", status, body));
    }

    let parsed: ListResponse = serde_json::from_str(&body)
        .map_err(|e| format!("Failed to parse webhook response: {} | Body: {}", e, &body[..body.len().min(200)]))?;

    if !parsed.ok {
        return Err(parsed.error.unwrap_or_else(|| "Webhook returned ok=false".to_string()));
    }

    println!("✅ Fetched {} rows from sheet", parsed.rows.len());
    Ok(parsed.rows)
}
