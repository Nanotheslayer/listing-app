use serde::{Deserialize, Serialize};
use reqwest;
use std::collections::HashMap;
use rand::Rng;
use std::io::Read;
use flate2::read::GzDecoder;

#[derive(Debug, Serialize, Deserialize)]
pub struct G2GAuthTokens {
    pub user_id: String,
    pub refresh_token: String,
    pub long_lived_token: String,
    pub active_device_token: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct RefreshResponse {
    payload: RefreshPayload,
}

#[derive(Debug, Serialize, Deserialize)]
struct RefreshPayload {
    access_token: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct SearchResponse {
    code: i32,
    payload: SearchPayload,
}

#[derive(Debug, Serialize, Deserialize)]
struct SearchPayload {
    results: Vec<SearchResult>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SearchResult {
    converted_unit_price: f64,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    title: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SkinPrice {
    pub skin_name: String,
    pub price: String,
}

pub struct G2GApiClient {
    client: reqwest::Client,
    base_url: String,
    current_token: Option<String>,
    session_id: String,
}

impl G2GApiClient {
    pub fn new() -> Self {
        // Генерируем случайный session_id
        let session_id = Self::generate_session_id();

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36")
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            client,
            base_url: "https://sls.g2g.com".to_string(),
            current_token: None,
            session_id,
        }
    }

    fn generate_session_id() -> String {
        let mut rng = rand::thread_rng();
        (0..32)
            .map(|_| format!("{:x}", rng.gen_range(0..16)))
            .collect()
    }

    fn get_browser_headers(&self, with_auth: bool) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();

        // Основные заголовки браузера
        headers.insert("Accept", "application/json, text/plain, */*".parse().unwrap());
        headers.insert("Accept-Language", "en-US,en;q=0.9".parse().unwrap());
        headers.insert("Accept-Encoding", "gzip, deflate, br".parse().unwrap());
        headers.insert("Connection", "keep-alive".parse().unwrap());
        headers.insert("Content-Type", "application/json".parse().unwrap());
        headers.insert("Cache-Control", "no-cache".parse().unwrap());
        headers.insert("Pragma", "no-cache".parse().unwrap());
        headers.insert("Origin", "https://www.g2g.com".parse().unwrap());

        // Referer с session_id
        let referer = format!("https://www.g2g.com/sellerhub?session_id={}", self.session_id);
        headers.insert("Referer", referer.parse().unwrap());

        // Sec-CH заголовки (Chrome)
        headers.insert("Sec-Ch-Ua", "\"Not A(Brand\";v=\"99\", \"Google Chrome\";v=\"123\", \"Chromium\";v=\"123\"".parse().unwrap());
        headers.insert("Sec-Ch-Ua-Mobile", "?0".parse().unwrap());
        headers.insert("Sec-Ch-Ua-Platform", "\"Windows\"".parse().unwrap());

        // Sec-Fetch заголовки
        headers.insert("Sec-Fetch-Dest", "empty".parse().unwrap());
        headers.insert("Sec-Fetch-Mode", "cors".parse().unwrap());
        headers.insert("Sec-Fetch-Site", "same-site".parse().unwrap());

        // Authorization
        if with_auth {
            if let Some(token) = &self.current_token {
                headers.insert("Authorization", token.parse().unwrap());
            }
        } else {
            headers.insert("Authorization", "null".parse().unwrap());
        }

        headers
    }

    pub async fn refresh_token(&mut self, tokens: &G2GAuthTokens) -> Result<String, String> {
        println!("🔄 Refreshing G2G token...");

        // Человекоподобная задержка перед запросом
        let delay_ms = rand::thread_rng().gen_range(1500..2500);
        tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;

        let url = format!("{}/user/refresh_access", self.base_url);

        let mut body = HashMap::new();
        body.insert("user_id", &tokens.user_id);
        body.insert("refresh_token", &tokens.refresh_token);
        body.insert("active_device_token", &tokens.active_device_token);
        body.insert("long_lived_token", &tokens.long_lived_token);

        let headers = self.get_browser_headers(false);

        println!("📤 Sending refresh request to G2G...");
        println!("   User ID: {}", tokens.user_id);

        let response = self.client
            .post(&url)
            .headers(headers)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        let status = response.status();
        println!("📥 Response status: {}", status);

        if status.is_success() {
            let json: RefreshResponse = response.json().await
                .map_err(|e| format!("Failed to parse response: {}", e))?;

            self.current_token = Some(json.payload.access_token.clone());
            println!("✅ Token refreshed successfully");
            Ok(json.payload.access_token)
        } else {
            let error_body = response.text().await.unwrap_or_else(|_| "Unable to read error body".to_string());
            println!("❌ Failed to refresh token: {} - {}", status, error_body);
            Err(format!("Failed to refresh token: {} - {}", status, error_body))
        }
    }

    pub async fn fetch_skin_price(&mut self, skin_name: &str, server: &str, tokens: &G2GAuthTokens) -> Result<String, String> {
        self.fetch_skin_price_impl(skin_name, server, tokens, false).await
    }

    fn fetch_skin_price_impl<'a>(
        &'a mut self,
        skin_name: &'a str,
        server: &'a str,
        tokens: &'a G2GAuthTokens,
        is_retry: bool,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, String>> + Send + 'a>> {
        Box::pin(async move {
            // Ensure we have a valid token
            if self.current_token.is_none() {
                println!("🔑 No token available, refreshing...");
                self.refresh_token(tokens).await?;
            }

            // Человекоподобная задержка между запросами
            let delay_ms = rand::thread_rng().gen_range(1000..2000);
            tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;

            // Server filter mapping (из Python скрипта)
            let server_filter = match server {
                "EUW" | "Europe West" => "319340f0:65ec9642%7Ce80c30d1:304244a1",
                "EUNE" | "Europe Nordic & East" => "319340f0:65ec9642%7Ce80c30d1:1a87dd85",
                "NA" | "North America" | "NA1" => "319340f0:65ec9642%7Ce80c30d1:e2f2c55b",
                "BR" | "BR1" | "Brazil" => "e80c30d1:31e5d298%7C319340f0:65ec9642",
                "LAN" => "319340f0:65ec9642%7Ce80c30d1:302ba1e6",
                "LAS" => "319340f0:65ec9642%7Ce80c30d1:d6ed5ab1",
                "OCE" | "Oceania" => "319340f0:65ec9642%7Ce80c30d1:e35ad6c4",
                "TR" | "Turkey" => "319340f0:65ec9642%7Ce80c30d1:5f8be29a",
                "RU" | "Russia" => "319340f0:65ec9642%7Ce80c30d1:d94d8d49",
                "JP" | "Japan" => "319340f0:65ec9642%7Ce80c30d1:8b6a5b8e",
                "KR" | "Korea" => "319340f0:65ec9642%7Ce80c30d1:a7bb0eb5",
                _ => "319340f0:65ec9642%7Ce80c30d1:1a87dd85", // Default to EUNE
            };

            let encoded_skin = urlencoding::encode(skin_name);
            let search_url = format!(
                "{}/offer/search?seo_term=league-of-legends-account&q={}&sort=lowest_price&filter_attr={}&page_size=48&currency=USD&country=RU&include_localization=0",
                self.base_url, encoded_skin, server_filter
            );

            println!("🔍 Searching for: {} on server {}", skin_name, server);

            let headers = self.get_browser_headers(true);

            let response = self.client
                .get(&search_url)
                .headers(headers)
                .send()
                .await
                .map_err(|e| format!("Search request failed: {}", e))?;

            let status = response.status();
            println!("   📥 Response status: {}", status);

            if status == 401 && !is_retry {
                println!("⚠️  Token expired (401), refreshing and retrying...");
                self.current_token = None;
                self.refresh_token(tokens).await?;
                return self.fetch_skin_price_impl(skin_name, server, tokens, true).await;
            }

            if !status.is_success() {
                return Err(format!("Search failed with status: {}", status));
            }

            // Проверяем Content-Encoding в ответе
            if let Some(encoding) = response.headers().get("content-encoding") {
                println!("   📦 Content-Encoding: {:?}", encoding);
            }

            // Пробуем получить байты и декодировать
            let bytes = response.bytes().await
                .map_err(|e| format!("Failed to read response bytes: {}", e))?;

            println!("   📊 Response size: {} bytes", bytes.len());

            // Проверяем gzip сигнатуру (1f 8b)
            let decoded_bytes = if bytes.len() >= 2 && bytes[0] == 0x1f && bytes[1] == 0x8b {
                println!("   🗜️  Decompressing gzip response...");
                let mut decoder = GzDecoder::new(&bytes[..]);
                let mut decoded = Vec::new();
                decoder.read_to_end(&mut decoded)
                    .map_err(|e| format!("Failed to decompress gzip: {}", e))?;
                println!("   ✅ Decompressed to {} bytes", decoded.len());
                decoded
            } else {
                println!("   📄 Response is not gzipped");
                bytes.to_vec()
            };

            // Пробуем распарсить как JSON
            let json: SearchResponse = serde_json::from_slice(&decoded_bytes)
                .map_err(|e| {
                    let preview = String::from_utf8_lossy(&decoded_bytes[..decoded_bytes.len().min(200)]);
                    format!("Failed to parse JSON: {} | Preview: {}", e, preview)
                })?;

            println!("   ✅ Successfully parsed response, code: {}", json.code);

            println!("   📦 Total results returned: {}", json.payload.results.len());

            // Логируем первые несколько результатов для отладки
            if json.payload.results.len() > 0 {
                println!("   🔍 Sample results (showing first 3):");
                for (i, result) in json.payload.results.iter().take(3).enumerate() {
                    println!("      ━━━ Result #{} ━━━", i+1);
                    println!("      💵 Price: ${:.2}", result.converted_unit_price);
                    if let Some(title) = &result.title {
                        println!("      📌 Title: {}", title);
                    } else {
                        println!("      📌 Title: [none]");
                    }
                    if let Some(desc) = &result.description {
                        let preview = if desc.len() > 150 {
                            format!("{}...", &desc[..150])
                        } else {
                            desc.clone()
                        };
                        println!("      📝 Description: {}", preview);
                    } else {
                        println!("      📝 Description: [none]");
                    }
                }
            }

            // Find minimum price from matching results
            let skin_lower = skin_name.to_lowercase();
            println!("   🎯 Looking for skin: '{}'", skin_lower);

            let matching_results: Vec<_> = json.payload.results
                .iter()
                .enumerate()
                .filter(|(idx, result)| {
                    let desc_match = result.description.as_ref()
                        .map(|d| d.to_lowercase().contains(&skin_lower))
                        .unwrap_or(false);
                    let title_match = result.title.as_ref()
                        .map(|t| t.to_lowercase().contains(&skin_lower))
                        .unwrap_or(false);

                    let matches = desc_match || title_match;
                    if matches {
                        println!("      ✓ Match found at index {}", idx);
                    }
                    matches
                })
                .map(|(_, result)| result)
                .collect();

            println!("   📊 Matching results: {} out of {}", matching_results.len(), json.payload.results.len());

            if matching_results.is_empty() {
                // Если ничего не нашли с фильтром, возвращаем минимальную цену из всех результатов
                if json.payload.results.len() > 0 {
                    let min_price = json.payload.results
                        .iter()
                        .map(|r| r.converted_unit_price)
                        .fold(f64::INFINITY, f64::min);
                    println!("   ⚠️  No exact match for '{}', using minimum from all results: ${:.2}", skin_name, min_price);
                    Ok(format!("~${:.2}", min_price))
                } else {
                    println!("   ❌ No offers found at all");
                    Ok("No offers".to_string())
                }
            } else {
                let min_price = matching_results
                    .iter()
                    .map(|r| r.converted_unit_price)
                    .fold(f64::INFINITY, f64::min);
                println!("   ✅ Found exact match: ${:.2} (from {} matching offers)", min_price, matching_results.len());
                Ok(format!("${:.2}", min_price))
            }
        })
    }
}
