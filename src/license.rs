use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct StoredLicense {
    pub license_key: String,
    // Persisted for file-format symmetry, but the launcher always regenerates
    // the fingerprint at runtime instead of trusting the stored value.
    #[allow(dead_code)]
    pub fingerprint: String,
    pub tiktok_username: String,
}

fn app_data_dir() -> PathBuf {
    let base = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| String::from("."));
    PathBuf::from(base).join("OnlyClimb")
}

fn license_path() -> PathBuf {
    app_data_dir().join("license.key")
}

fn legacy_license_path() -> PathBuf {
    std::env::current_exe()
        .unwrap_or_else(|_| PathBuf::from("."))
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join("license.key")
}

fn parse_license_content(content: &str) -> Option<StoredLicense> {
    let mut license_key = String::new();
    let mut fingerprint = String::new();
    let mut tiktok_username = String::new();

    for line in content.lines() {
        if let Some(v) = line.strip_prefix("license_key=") {
            license_key = v.trim().to_string();
        } else if let Some(v) = line.strip_prefix("fingerprint=") {
            fingerprint = v.trim().to_string();
        } else if let Some(v) = line.strip_prefix("tiktok_username=") {
            tiktok_username = v.trim().to_string();
        }
    }

    if license_key.is_empty() {
        return None;
    }

    Some(StoredLicense {
        license_key,
        fingerprint,
        tiktok_username,
    })
}

pub fn read_license() -> Option<StoredLicense> {
    let path = license_path();
    if let Ok(content) = fs::read_to_string(&path) {
        return parse_license_content(&content);
    }

    let legacy = legacy_license_path();
    if let Ok(content) = fs::read_to_string(&legacy) {
        return parse_license_content(&content);
    }

    None
}

pub fn save_license(key: &str, fp: &str, tiktok: &str) -> Result<(), String> {
    let dir = app_data_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("Erreur création dossier AppData: {e}"))?;
    let content = format!("license_key={key}\nfingerprint={fp}\ntiktok_username={tiktok}\n");
    fs::write(license_path(), content).map_err(|e| format!("Erreur d'écriture license.key: {e}"))
}

/// Removes the persisted license file. Currently unused (no "sign out" UI yet),
/// kept as part of this module's small storage API.
#[allow(dead_code)]
pub fn delete_license() {
    let _ = fs::remove_file(license_path());
    let _ = fs::remove_file(legacy_license_path());
}
