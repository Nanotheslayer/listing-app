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

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateJobRequest {
    pub offer_id: String,
    pub relation_id: String,
    pub seller_id: String,
    pub file_type: String,
    pub files: Vec<String>,
    pub brand_id: String,
    pub service_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct CreateJobResponse {
    code: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExternalImage {
    pub image_name: String,
    pub image_url: String,
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

    // Accessors used by the parallel price fetch pipeline in main.rs.
    // These expose client internals so that concurrent tasks can run without
    // holding the top-level Mutex<G2GApiClient> for the whole batch.
    pub fn http_client(&self) -> reqwest::Client {
        self.client.clone()
    }

    pub fn base_url_cloned(&self) -> String {
        self.base_url.clone()
    }

    pub fn session_id_cloned(&self) -> String {
        self.session_id.clone()
    }

    pub fn current_token(&self) -> Option<String> {
        self.current_token.clone()
    }

    pub fn set_current_token(&mut self, token: Option<String>) {
        self.current_token = token;
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
                "EUW" | "Europe West" => "e80c30d1:304244a1%7C319340f0:65ec9642",
                "EUNE" | "Europe Nordic & East" => "e80c30d1:1a87dd85%7C319340f0:65ec9642",
                "NA" | "North America" | "NA1" => "e80c30d1:e2f2c55b%7C319340f0:65ec9642",
                "BR" | "BR1" | "Brazil" => "e80c30d1:31e5d298%7C319340f0:65ec9642",
                "LAN" => "e80c30d1:302ba1e6%7C319340f0:65ec9642",
                "LAS" => "e80c30d1:f28899f5%7C319340f0:65ec9642",
                "OCE" | "Oceania" => "e80c30d1:5c030fef%7C319340f0:65ec9642",
                "TR" | "Turkey" => "e80c30d1:2247e703%7C319340f0:65ec9642",
                "RU" | "Russia" => "319340f0:65ec9642%7Ce80c30d1:d94d8d49",
                "JP" | "Japan" => "e80c30d1:e9926686%7C319340f0:65ec9642",
                "KR" | "Korea" => "319340f0:65ec9642%7Ce80c30d1:a7bb0eb5",
                _ => "e80c30d1:1a87dd85%7C319340f0:65ec9642", // Default to EUNE
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

// Структуры для создания оффера
#[derive(Debug, Serialize, Deserialize)]
pub struct OfferAttribute {
    pub collection_id: String,
    pub dataset_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateOfferRequest {
    pub service_id: String,
    pub brand_id: String,
    pub offer_type: String,
    pub seller_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct CreateOfferResponse {
    code: i32,
    payload: CreateOfferPayload,
}

#[derive(Debug, Serialize, Deserialize)]
struct CreateOfferPayload {
    offer_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateOfferRequest {
    pub seller_id: String,
    pub offer_id: String,
    pub delivery_method_ids: Vec<String>,
    pub delivery_speed: String,
    pub delivery_speed_details: Vec<DeliverySpeed>,
    pub qty: i32,
    pub currency: String,
    pub min_qty: i32,
    pub low_stock_alert_qty: i32,
    pub sales_territory_settings: SalesTerritory,
    pub title: String,
    pub description: String,
    pub offer_attributes: Vec<OfferAttribute>,
    pub external_images_mapping: Vec<ExternalImage>,
    pub unit_price: f64,
    pub other_pricing: Vec<String>,
    pub wholesale_details: Vec<String>,
    pub other_wholesale_details: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeliverySpeed {
    pub min: i32,
    pub max: i32,
    pub delivery_time: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SalesTerritory {
    pub settings_type: String,
    pub countries: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct UpdateOfferResponse {
    code: i32,
    #[serde(default)]
    payload: Option<UpdateOfferPayload>,
}

#[derive(Debug, Serialize, Deserialize)]
struct UpdateOfferPayload {
    relation_id: String,
    // остальные поля опциональны
    #[serde(default)]
    offer_id: Option<String>,
}

impl G2GApiClient {
    // Создать пустой оффер и получить ID
    pub async fn create_offer_id(&mut self, tokens: &G2GAuthTokens) -> Result<String, String> {
        println!("📝 Creating empty offer to get ID...");

        // ВАЖНО: Принудительно обновляем токен перед созданием оффера
        println!("🔄 Refreshing token before creating offer...");
        self.refresh_token(tokens).await?;

        let url = format!("{}/offer", self.base_url);

        let request = CreateOfferRequest {
            service_id: "f6a1aba5-473a-4044-836a-8968bbab16d7".to_string(),
            brand_id: "lgc_game_22666".to_string(),
            offer_type: "public".to_string(),
            seller_id: tokens.user_id.clone(),
        };

        let headers = self.get_browser_headers(true);

        let response = self.client
            .post(&url)
            .headers(headers)
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("Failed to create offer: {}", e))?;

        let status = response.status();
        println!("📥 Create offer response status: {}", status);

        if status == 401 {
            // Если все еще 401, пробуем еще раз с новым токеном
            println!("⚠️  Got 401, refreshing token and retrying...");
            self.current_token = None;
            self.refresh_token(tokens).await?;

            let headers = self.get_browser_headers(true);
            let response = self.client
                .post(&url)
                .headers(headers)
                .json(&request)
                .send()
                .await
                .map_err(|e| format!("Failed to create offer (retry): {}", e))?;

            let status = response.status();
            println!("📥 Create offer response status (retry): {}", status);

            if !status.is_success() {
                let error_body = response.text().await.unwrap_or_else(|_| "Unable to read error".to_string());
                return Err(format!("Failed to create offer after retry: {} - {}", status, error_body));
            }

            let json: CreateOfferResponse = response.json().await
                .map_err(|e| format!("Failed to parse create offer response: {}", e))?;

            println!("✅ Offer ID created: {}", json.payload.offer_id);
            return Ok(json.payload.offer_id);
        }

        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_else(|_| "Unable to read error".to_string());
            return Err(format!("Failed to create offer: {} - {}", status, error_body));
        }

        let json: CreateOfferResponse = response.json().await
            .map_err(|e| format!("Failed to parse create offer response: {}", e))?;

        println!("✅ Offer ID created: {}", json.payload.offer_id);
        Ok(json.payload.offer_id)
    }

    // Обновить оффер с полными данными
    pub async fn update_offer(
        &mut self,
        offer_id: &str,
        title: &str,
        description: &str,
        price: f64,
        server: &str,
        rank: &str,
        champions_count: i32,
        skins_count: i32,
        screenshot_url: Option<&str>,
        tokens: &G2GAuthTokens,
    ) -> Result<String, String> {
        println!("📄 Updating offer {} with data...", offer_id);

        let url = format!("{}/offer/{}", self.base_url, offer_id);

        let offer_attributes = self.create_offer_attributes(server, rank, champions_count, skins_count);

        // Формируем массив изображений
        let external_images = if let Some(url) = screenshot_url {
            vec![ExternalImage {
                image_name: "1".to_string(),
                image_url: url.to_string(),
            }]
        } else {
            vec![]
        };

        let request = UpdateOfferRequest {
            seller_id: tokens.user_id.clone(),
            offer_id: offer_id.to_string(),
            delivery_method_ids: vec![],
            delivery_speed: "instant".to_string(),
            delivery_speed_details: vec![],
            qty: 1,
            currency: "USD".to_string(),
            min_qty: 1,
            low_stock_alert_qty: 0,
            sales_territory_settings: SalesTerritory {
                settings_type: "global".to_string(),
                countries: vec![],
            },
            title: title.to_string(),
            description: description.to_string(),
            offer_attributes,
            external_images_mapping: external_images,  // ← Теперь эта переменная существует
            unit_price: price,
            other_pricing: vec![],
            wholesale_details: vec![],
            other_wholesale_details: vec![],
        };

        let headers = self.get_browser_headers(true);

        let response = self.client
            .put(&url)
            .headers(headers)
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("Failed to update offer: {}", e))?;

        let status = response.status();
        println!("📥 Update offer response status: {}", status);

        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_else(|_| "Unable to read error".to_string());
            return Err(format!("Failed to update offer: {} - {}", status, error_body));
        }

        let bytes = response.bytes().await
            .map_err(|e| format!("Failed to read response bytes: {}", e))?;

        let decoded_bytes = if bytes.len() >= 2 && bytes[0] == 0x1f && bytes[1] == 0x8b {
            println!("🗜️ Decompressing gzip response...");
            let mut decoder = GzDecoder::new(&bytes[..]);
            let mut decoded = Vec::new();
            decoder.read_to_end(&mut decoded)
                .map_err(|e| format!("Failed to decompress gzip: {}", e))?;
            decoded
        } else {
            bytes.to_vec()
        };

        let response_text = String::from_utf8_lossy(&decoded_bytes);
        println!("📄 Update offer response body: {}", response_text);

        let json: UpdateOfferResponse = serde_json::from_slice(&decoded_bytes)
            .map_err(|e| format!("Failed to parse update response: {}", e))?;

        if json.code != 2000 {
            return Err(format!("Update offer returned code: {}", json.code));
        }

        let relation_id = json.payload
            .ok_or("No payload in update response")?
            .relation_id;

        println!("✅ Offer updated successfully! Relation ID: {}", relation_id);
        Ok(relation_id)
    }

    // Новая функция для загрузки softpin данных
    pub async fn upload_softpin_data(
        &mut self,
        offer_id: &str,
        relation_id: &str,
        softpin_content: &str,
        tokens: &G2GAuthTokens,
    ) -> Result<(), String> {
        println!("📤 Uploading softpin...");
        println!("🔍 Softpin content length: {}", softpin_content.len());
        println!("🔍 First 300 chars: {}", &softpin_content[..softpin_content.len().min(300)]);

        let url = format!("{}/inventory/softpin", self.base_url);

        // Используем serde_json для правильной сериализации
        let request_body = serde_json::json!({
            "offer_id": offer_id,
            "relation_id": relation_id,
            "softpin": softpin_content,  // serde_json автоматически экранирует правильно
            "seller_id": tokens.user_id
        });

        let json_body = serde_json::to_string(&request_body)
            .map_err(|e| format!("Failed to serialize request: {}", e))?;

        println!("📤 Final JSON length: {}", json_body.len());
        println!("📤 JSON preview (first 500 chars):");
        println!("{}", &json_body[..json_body.len().min(500)]);

        let headers = self.get_browser_headers(true);

        let response = self.client
            .post(&url)
            .headers(headers)
            .header("content-type", "application/json")
            .body(json_body)
            .send()
            .await
            .map_err(|e| format!("Failed to upload softpin: {}", e))?;

        let status = response.status();
        println!("📥 Response status: {}", status);

        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_else(|_| "Unable to read".to_string());
            return Err(format!("Upload failed {}: {}", status, error_body));
        }

        let bytes = response.bytes().await.map_err(|e| format!("Read error: {}", e))?;
        let decoded_bytes = if bytes.len() >= 2 && bytes[0] == 0x1f && bytes[1] == 0x8b {
            let mut decoder = GzDecoder::new(&bytes[..]);
            let mut decoded = Vec::new();
            decoder.read_to_end(&mut decoded).map_err(|e| format!("Decompress error: {}", e))?;
            decoded
        } else {
            bytes.to_vec()
        };

        let response_text = String::from_utf8_lossy(&decoded_bytes);
        println!("📄 Response: {}", response_text);

        let json: serde_json::Value = serde_json::from_slice(&decoded_bytes)
            .map_err(|e| format!("Parse error: {}", e))?;

        if json["code"].as_i64() != Some(2000) {
            return Err(format!("Failed with code: {}", json["code"]));
        }

        println!("✅ Softpin uploaded!");
        Ok(())
    }

    // Загрузить данные аккаунта (softpin) и создать job
    pub async fn upload_account_data(
        &mut self,
        offer_id: &str,
        relation_id: &str,
        softpin_content: &str,
        tokens: &G2GAuthTokens,
    ) -> Result<(), String> {
        println!("📦 Processing account data for offer {}...", offer_id);

        // Шаг 1: Загрузить softpin данные
        self.upload_softpin_data(offer_id, relation_id, softpin_content, tokens).await?;

        // Задержка между запросами
        let delay_ms = rand::thread_rng().gen_range(500..1000);
        tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;

        // Шаг 2: Создать job для обработки
        println!("📋 Creating processing job...");

        let job_url = format!("{}/inventory/job", self.base_url);

        // Генерируем имя CSV файла
        let csv_filename = format!("{}/{}.csv", offer_id, offer_id);

        let job_request = serde_json::json!({
            "offer_id": offer_id,
            "relation_id": relation_id,
            "seller_id": tokens.user_id,
            "file_type": "csv",
            "files": [csv_filename],
            "brand_id": "lgc_game_22666",
            "service_id": "f6a1aba5-473a-4044-836a-8968bbab16d7",
        });

        println!("📤 Job request body: {}", serde_json::to_string_pretty(&job_request).unwrap());

        let headers = self.get_browser_headers(true);

        let response = self.client
            .post(&job_url)
            .headers(headers)
            .json(&job_request)
            .send()
            .await
            .map_err(|e| format!("Failed to create job: {}", e))?;

        let status = response.status();
        println!("📥 Create job response status: {}", status);

        let bytes = response.bytes().await
            .map_err(|e| format!("Failed to read job response bytes: {}", e))?;

        let decoded_bytes = if bytes.len() >= 2 && bytes[0] == 0x1f && bytes[1] == 0x8b {
            let mut decoder = GzDecoder::new(&bytes[..]);
            let mut decoded = Vec::new();
            decoder.read_to_end(&mut decoded)
                .map_err(|e| format!("Failed to decompress: {}", e))?;
            decoded
        } else {
            bytes.to_vec()
        };

        let response_text = String::from_utf8_lossy(&decoded_bytes);
        println!("📄 Job response body: {}", response_text);

        if !status.is_success() {
            return Err(format!("Job creation failed with status {}: {}", status, response_text));
        }

        let job_json: serde_json::Value = serde_json::from_slice(&decoded_bytes)
            .map_err(|e| format!("Failed to parse job response: {}", e))?;

        if job_json["code"].as_i64() == Some(2000) {
            println!("✅ Account data processing job created successfully!");
            Ok(())
        } else {
            Err(format!("Create job returned code: {}", job_json["code"]))
        }
    }


    // Создать полный оффер с загрузкой данных (трехстапный процесс)
    pub async fn create_full_offer_with_data(
        &mut self,
        title: &str,
        description: &str,
        price: f64,
        server: &str,
        rank: &str,
        champions_count: i32,
        skins_count: i32,
        softpin_content: &str,
        screenshot_url: Option<&str>,
        tokens: &G2GAuthTokens,
    ) -> Result<String, String> {
        println!("🎯 Starting full offer creation with data upload...");

        // Шаг 1: Создать пустой оффер
        let offer_id = self.create_offer_id(tokens).await?;

        let delay_ms = rand::thread_rng().gen_range(1500..2500);
        tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;

        // Шаг 2: Обновить оффер и получить relation_id
        let relation_id = self.update_offer(
            &offer_id,
            title,
            description,
            price,
            server,
            rank,
            champions_count,
            skins_count,
            screenshot_url,
            tokens,
        ).await?;

        let delay_ms = rand::thread_rng().gen_range(1500..2500);
        tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;

        // Шаг 3: Загрузить данные аккаунта используя relation_id
        self.upload_account_data(&offer_id, &relation_id, softpin_content, tokens).await?;

        println!("🎉 Full offer with data created successfully! ID: {}", offer_id);
        Ok(offer_id)
    }

    // Вспомогательные функции маппинга
    fn get_server_id(&self, server: &str) -> &str {
        match server.to_uppercase().as_str() {
            "EUNE" => "1a87dd85",
            "EUW" => "304244a1",
            "NA" | "NA1" => "e2f2c55b",
            "BR" | "BR1" => "31e5d298",
            "LAN" => "302ba1e6",
            "LAS" => "f28899f5",
            "OCE" => "5c030fef",
            "TR" => "2247e703",
            "RU" => "d94d8d49",
            "JP" => "e9926686",
            "KR" => "a7bb0eb5",
            _ => "1a87dd85", // Default EUNE
        }
    }

    fn get_rank_id(&self, rank: &str) -> &str {
        match rank {
            "Unranked" | "Has games" => "dc514fdf",
            "Iron" => "64174ec3",
            "Bronze" => "bf08fd78",
            "Silver" => "405dc68e",
            "Gold" => "55077770",
            "Platinum" => "8a193251",
            "Emerald" => "297e3197",
            "Diamond" => "3b06cc4c",
            "Master" => "08f99b44",
            "Grandmaster" => "447eb997",
            "Challenger" => "99197149",
            _ => "dc514fdf", // Default Unranked
        }
    }

    fn get_champions_id(&self, count: i32) -> &str {
        if count > 159 {
            "3ee17abb"
        } else if count > 129 {
            "dc9b65bb"
        } else if count > 99 {
            "2ea03f75"
        } else if count > 49 {
            "7bbf537c"
        } else if count > 29 {
            "191cd6d7"
        } else if count > 9 {
            "b03ce3d1"
        } else {
            "b5d60c4b"
        }
    }

    fn get_skins_id(&self, count: i32) -> &str {
        if count > 999 {
            "da83ec6e"
        } else if count > 499 {
            "32895a53"
        } else if count > 299 {
            "bbe13228"
        } else if count > 99 {
            "70f8019b"
        } else if count > 49 {
            "c1721794"
        } else if count > 9 {
            "4be5718c"
        } else {
            "ce97df6f"
        }
    }

    fn create_offer_attributes(
        &self,
        server: &str,
        rank: &str,
        champions_count: i32,
        skins_count: i32,
    ) -> Vec<OfferAttribute> {
        vec![
            // Server
            OfferAttribute {
                collection_id: "e80c30d1".to_string(),
                dataset_id: self.get_server_id(server).to_string(),
            },
            // Type (Account for League of Legends)
            OfferAttribute {
                collection_id: "319340f0".to_string(),
                dataset_id: "65ec9642".to_string(),
            },
            // Rank
            OfferAttribute {
                collection_id: "eb7040e2".to_string(),
                dataset_id: self.get_rank_id(rank).to_string(),
            },
            // Champions count
            OfferAttribute {
                collection_id: "04862150".to_string(),
                dataset_id: self.get_champions_id(champions_count).to_string(),
            },
            // Skins count
            OfferAttribute {
                collection_id: "962f619a".to_string(),
                dataset_id: self.get_skins_id(skins_count).to_string(),
            },
        ]
    }
}

// --- Shared helpers used by the parallel price-fetch pipeline ---

#[derive(Debug)]
pub enum SkinSearchError {
    Unauthorized,
    Other(String),
}

fn build_browser_headers_shared(
    session_id: &str,
    access_token: Option<&str>,
) -> reqwest::header::HeaderMap {
    let mut headers = reqwest::header::HeaderMap::new();

    headers.insert("Accept", "application/json, text/plain, */*".parse().unwrap());
    headers.insert("Accept-Language", "en-US,en;q=0.9".parse().unwrap());
    headers.insert("Accept-Encoding", "gzip, deflate, br".parse().unwrap());
    headers.insert("Connection", "keep-alive".parse().unwrap());
    headers.insert("Content-Type", "application/json".parse().unwrap());
    headers.insert("Cache-Control", "no-cache".parse().unwrap());
    headers.insert("Pragma", "no-cache".parse().unwrap());
    headers.insert("Origin", "https://www.g2g.com".parse().unwrap());

    let referer = format!("https://www.g2g.com/sellerhub?session_id={}", session_id);
    headers.insert("Referer", referer.parse().unwrap());

    headers.insert("Sec-Ch-Ua", "\"Not A(Brand\";v=\"99\", \"Google Chrome\";v=\"123\", \"Chromium\";v=\"123\"".parse().unwrap());
    headers.insert("Sec-Ch-Ua-Mobile", "?0".parse().unwrap());
    headers.insert("Sec-Ch-Ua-Platform", "\"Windows\"".parse().unwrap());

    headers.insert("Sec-Fetch-Dest", "empty".parse().unwrap());
    headers.insert("Sec-Fetch-Mode", "cors".parse().unwrap());
    headers.insert("Sec-Fetch-Site", "same-site".parse().unwrap());

    match access_token {
        Some(token) => {
            headers.insert("Authorization", token.parse().unwrap());
        }
        None => {
            headers.insert("Authorization", "null".parse().unwrap());
        }
    }

    headers
}

fn map_server_filter(server: &str) -> &'static str {
    match server {
        "EUW" | "Europe West" => "e80c30d1:304244a1%7C319340f0:65ec9642",
        "EUNE" | "Europe Nordic & East" => "e80c30d1:1a87dd85%7C319340f0:65ec9642",
        "NA" | "North America" | "NA1" => "e80c30d1:e2f2c55b%7C319340f0:65ec9642",
        "BR" | "BR1" | "Brazil" => "e80c30d1:31e5d298%7C319340f0:65ec9642",
        "LAN" => "e80c30d1:302ba1e6%7C319340f0:65ec9642",
        "LAS" => "e80c30d1:f28899f5%7C319340f0:65ec9642",
        "OCE" | "Oceania" => "e80c30d1:5c030fef%7C319340f0:65ec9642",
        "TR" | "Turkey" => "e80c30d1:2247e703%7C319340f0:65ec9642",
        "RU" | "Russia" => "319340f0:65ec9642%7Ce80c30d1:d94d8d49",
        "JP" | "Japan" => "e80c30d1:e9926686%7C319340f0:65ec9642",
        "KR" | "Korea" => "319340f0:65ec9642%7Ce80c30d1:a7bb0eb5",
        _ => "e80c30d1:1a87dd85%7C319340f0:65ec9642",
    }
}

// Single-skin search that takes its state by value — safe to run concurrently.
pub async fn search_skin_price_shared(
    http_client: &reqwest::Client,
    base_url: &str,
    session_id: &str,
    access_token: &str,
    skin_name: &str,
    server: &str,
) -> Result<String, SkinSearchError> {
    let server_filter = map_server_filter(server);
    let encoded_skin = urlencoding::encode(skin_name);
    let search_url = format!(
        "{}/offer/search?seo_term=league-of-legends-account&q={}&sort=lowest_price&filter_attr={}&page_size=48&currency=USD&country=RU&include_localization=0",
        base_url, encoded_skin, server_filter
    );

    let headers = build_browser_headers_shared(session_id, Some(access_token));

    let response = http_client
        .get(&search_url)
        .headers(headers)
        .send()
        .await
        .map_err(|e| SkinSearchError::Other(format!("Search request failed: {}", e)))?;

    let status = response.status();

    if status == 401 {
        return Err(SkinSearchError::Unauthorized);
    }

    if !status.is_success() {
        return Err(SkinSearchError::Other(format!(
            "Search failed with status: {}",
            status
        )));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| SkinSearchError::Other(format!("Failed to read response bytes: {}", e)))?;

    let decoded_bytes = if bytes.len() >= 2 && bytes[0] == 0x1f && bytes[1] == 0x8b {
        let mut decoder = GzDecoder::new(&bytes[..]);
        let mut decoded = Vec::new();
        decoder
            .read_to_end(&mut decoded)
            .map_err(|e| SkinSearchError::Other(format!("Failed to decompress gzip: {}", e)))?;
        decoded
    } else {
        bytes.to_vec()
    };

    let json: SearchResponse = serde_json::from_slice(&decoded_bytes).map_err(|e| {
        let preview = String::from_utf8_lossy(&decoded_bytes[..decoded_bytes.len().min(200)]);
        SkinSearchError::Other(format!("Failed to parse JSON: {} | Preview: {}", e, preview))
    })?;

    let skin_lower = skin_name.to_lowercase();

    let matching_results: Vec<_> = json
        .payload
        .results
        .iter()
        .filter(|result| {
            let desc_match = result
                .description
                .as_ref()
                .map(|d| d.to_lowercase().contains(&skin_lower))
                .unwrap_or(false);
            let title_match = result
                .title
                .as_ref()
                .map(|t| t.to_lowercase().contains(&skin_lower))
                .unwrap_or(false);
            desc_match || title_match
        })
        .collect();

    if matching_results.is_empty() {
        if !json.payload.results.is_empty() {
            let min_price = json
                .payload
                .results
                .iter()
                .map(|r| r.converted_unit_price)
                .fold(f64::INFINITY, f64::min);
            Ok(format!("~${:.2}", min_price))
        } else {
            Ok("No offers".to_string())
        }
    } else {
        let min_price = matching_results
            .iter()
            .map(|r| r.converted_unit_price)
            .fold(f64::INFINITY, f64::min);
        Ok(format!("${:.2}", min_price))
    }
}

// Shared refresh that doesn't mutate a client — returns the new access token.
pub async fn refresh_access_token_shared(
    http_client: &reqwest::Client,
    base_url: &str,
    session_id: &str,
    tokens: &G2GAuthTokens,
) -> Result<String, String> {
    let url = format!("{}/user/refresh_access", base_url);

    let mut body = HashMap::new();
    body.insert("user_id", &tokens.user_id);
    body.insert("refresh_token", &tokens.refresh_token);
    body.insert("active_device_token", &tokens.active_device_token);
    body.insert("long_lived_token", &tokens.long_lived_token);

    let headers = build_browser_headers_shared(session_id, None);

    let response = http_client
        .post(&url)
        .headers(headers)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Refresh request failed: {}", e))?;

    let status = response.status();

    if status.is_success() {
        let json: RefreshResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse refresh response: {}", e))?;
        Ok(json.payload.access_token)
    } else {
        let error_body = response
            .text()
            .await
            .unwrap_or_else(|_| "Unable to read error body".to_string());
        Err(format!(
            "Failed to refresh token: {} - {}",
            status, error_body
        ))
    }
}
