use serde_json::Value;

#[derive(Debug, Clone, Default)]
pub struct LicenseResponse {
    pub valid: bool,
    pub reason: String,
    pub message: String,
    pub license: Option<License>,
}

#[derive(Debug, Clone, Default)]
pub struct License {
    pub tiktok_username: String,
    pub status: String,
    pub created_at: String,
    pub status_changed_at: String,
    pub expires_at: String,
    pub expires_in_days: Option<i64>,
    pub last_check: String,
    pub fingerprint_bound: bool,
    pub fingerprint_match: bool,
}

fn string_or_default(v: &Value, key: &str) -> String {
    v.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_default()
}

fn bool_or_default(v: &Value, key: &str) -> bool {
    v.get(key).and_then(|v| v.as_bool()).unwrap_or(false)
}

fn opt_i64_or_default(v: &Value, key: &str) -> Option<i64> {
    v.get(key).and_then(|v| v.as_i64())
}

impl From<Value> for LicenseResponse {
    fn from(v: Value) -> Self {
        let valid = bool_or_default(&v, "valid");
        let reason = string_or_default(&v, "reason");
        let message = string_or_default(&v, "message");
        let license = v.get("license").cloned().map(License::from);

        LicenseResponse {
            valid,
            reason,
            message,
            license,
        }
    }
}

impl From<Value> for License {
    fn from(v: Value) -> Self {
        License {
            tiktok_username: string_or_default(&v, "tiktok_username"),
            status: string_or_default(&v, "status"),
            created_at: string_or_default(&v, "created_at"),
            status_changed_at: string_or_default(&v, "status_changed_at"),
            expires_at: string_or_default(&v, "expires_at"),
            expires_in_days: opt_i64_or_default(&v, "expires_in_days"),
            last_check: string_or_default(&v, "last_check"),
            fingerprint_bound: bool_or_default(&v, "fingerprint_bound"),
            fingerprint_match: bool_or_default(&v, "fingerprint_match"),
        }
    }
}

pub async fn validate_license(
    license_key: &str,
    fingerprint: &str,
    tiktok_username: &str,
) -> Result<LicenseResponse, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .map_err(|e| format!("Client error: {e}"))?;

    let resp = client
        .get("https://panel.mystreamgame.com/api-license.php")
        .query(&[
            ("license_key", license_key),
            ("fingerprint", fingerprint),
            ("tiktok_username", tiktok_username),
        ])
        .send()
        .await
        .map_err(|e| format!("Network error: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Server returned {}", resp.status()));
    }

    let raw = resp
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {e}"))?;

    let val: Value =
        serde_json::from_str(&raw).map_err(|e| format!("JSON parse error: {e}\nBody: {raw}"))?;

    let body: LicenseResponse = val.into();
    Ok(body)
}
