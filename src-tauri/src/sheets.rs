use serde::Serialize;

/// Одна строка, отправляемая в Google-таблицу через веб-хук Apps Script.
#[derive(Debug, Serialize)]
pub struct SheetRow {
    pub username: String,
    pub offer_id: String,
    pub listed_date: String,
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
