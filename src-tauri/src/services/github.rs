// GitHub service - OAuth token exchange utilities
pub async fn exchange_code_for_token(
    code: &str,
    client_secret: &str,
    client_id: &str,
) -> Result<String, String> {
    let client = reqwest::Client::new();
    let response = client
        .post("https://github.com/login/oauth/access_token")
        .header("Accept", "application/json")
        .json(&serde_json::json!({
            "client_id": client_id,
            "client_secret": client_secret,
            "code": code,
        }))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let body: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;

    body["access_token"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| body["error_description"].as_str().unwrap_or("OAuth failed").to_string())
}
