use serde::{Deserialize, Serialize};
use reqwest;
use std::collections::HashMap;

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
    payload: SearchPayload,
}

#[derive(Debug, Serialize, Deserialize)]
struct SearchPayload {
    results: Vec<SearchResult>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SearchResult {
    converted_unit_price: f64,
    description: Option<String>,
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
}

impl G2GApiClient {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            client,
            base_url: "https://sls.g2g.com".to_string(),
            current_token: None,
        }
    }

    pub async fn refresh_token(&mut self, tokens: &G2GAuthTokens) -> Result<String, String> {
        let url = format!("{}/user/refresh_access", self.base_url);

        let mut body = HashMap::new();
        body.insert("user_id", &tokens.user_id);
        body.insert("refresh_token", &tokens.refresh_token);
        body.insert("active_device_token", &tokens.active_device_token);
        body.insert("long_lived_token", &tokens.long_lived_token);

        let response = self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/plain, */*")
            .header("Origin", "https://www.g2g.com")
            .header("Referer", "https://www.g2g.com/")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if response.status().is_success() {
            let json: RefreshResponse = response.json().await
                .map_err(|e| format!("Failed to parse response: {}", e))?;
            
            self.current_token = Some(json.payload.access_token.clone());
            Ok(json.payload.access_token)
        } else {
            Err(format!("Failed to refresh token: {}", response.status()))
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
                self.refresh_token(tokens).await?;
            }

            let token = self.current_token.as_ref()
                .ok_or("No valid token available")?
                .clone();

            // Server filter mapping
            let server_filter = match server {
                "EUW" => "319340f0:65ec9642%7Ce80c30d1:304244a1",
                "EUNE" => "319340f0:65ec9642%7Ce80c30d1:1a87dd85",
                "NA" => "319340f0:65ec9642%7Ce80c30d1:302ba1e6",
                "BR" => "319340f0:65ec9642%7Ce80c30d1:6c9f06c3",
                "LAN" => "319340f0:65ec9642%7Ce80c30d1:302ba1e6",
                "LAS" => "319340f0:65ec9642%7Ce80c30d1:d6ed5ab1",
                "OCE" => "319340f0:65ec9642%7Ce80c30d1:e35ad6c4",
                "TR" => "319340f0:65ec9642%7Ce80c30d1:5f8be29a",
                "RU" => "319340f0:65ec9642%7Ce80c30d1:d94d8d49",
                "JP" => "319340f0:65ec9642%7Ce80c30d1:8b6a5b8e",
                "KR" => "319340f0:65ec9642%7Ce80c30d1:a7bb0eb5",
                _ => "319340f0:65ec9642%7Ce80c30d1:304244a1", // Default to EUW
            };

            let encoded_skin = urlencoding::encode(skin_name);
            let search_url = format!(
                "{}/offer/search?seo_term=league-of-legends-account&q={}&sort=lowest_price&filter_attr={}&page_size=48&currency=USD&country=RU&include_localization=0",
                self.base_url, encoded_skin, server_filter
            );

            let response = self.client
                .get(&search_url)
                .header("Content-Type", "application/json")
                .header("Accept", "application/json, text/plain, */*")
                .header("Authorization", &token)
                .header("Origin", "https://www.g2g.com")
                .header("Referer", "https://www.g2g.com/")
                .send()
                .await
                .map_err(|e| format!("Search request failed: {}", e))?;

            if response.status() == 401 && !is_retry {
                // Token expired, refresh and retry once
                self.current_token = None;
                self.refresh_token(tokens).await?;
                return self.fetch_skin_price_impl(skin_name, server, tokens, true).await;
            }

            if !response.status().is_success() {
                return Err(format!("Search failed: {}", response.status()));
            }

            let json: SearchResponse = response.json().await
                .map_err(|e| format!("Failed to parse search response: {}", e))?;

            // Find minimum price from matching results
            let skin_lower = skin_name.to_lowercase();
            let prices: Vec<f64> = json.payload.results
                .iter()
                .filter(|result| {
                    let desc_match = result.description.as_ref()
                        .map(|d| d.to_lowercase().contains(&skin_lower))
                        .unwrap_or(false);
                    let title_match = result.title.as_ref()
                        .map(|t| t.to_lowercase().contains(&skin_lower))
                        .unwrap_or(false);
                    desc_match || title_match
                })
                .map(|r| r.converted_unit_price)
                .collect();

            if prices.is_empty() {
                Ok("No offers".to_string())
            } else {
                let min_price = prices.iter()
                    .cloned()
                    .fold(f64::INFINITY, f64::min);
                Ok(format!("${:.2}", min_price))
            }
        })
    }
}
