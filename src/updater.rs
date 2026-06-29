//! Built-in self-update, mirroring the AIO Message pattern but for a single
//! portable `.exe`: query the GitHub Releases API of the CDN repo, and if a
//! newer version is published, download its `.exe` asset and atomically swap
//! the running binary (`self-replace`), then relaunch.
//!
//! Active only for a real install (exe run from outside `…\target\`). In dev
//! (`cargo run`) the updater is inert and makes no network request.

const OWNER: &str = "liiando";
const REPO: &str = "launcher-oc";
const CURRENT: &str = env!("CARGO_PKG_VERSION");
const UA: &str = concat!("OnlyClimb-Launcher/", env!("CARGO_PKG_VERSION"));

/// Update status surfaced to the UI.
#[derive(Clone, Debug)]
pub enum UpdateState {
    /// Nothing to do / dev build → updater inert.
    Idle,
    /// Already on the latest version.
    UpToDate,
    /// A newer version is available (its version string).
    Available(String),
    /// Download + swap in progress (the app is about to relaunch).
    Installing,
    /// New binary is in place and relaunched → the UI must exit now.
    ReadyToQuit,
    /// Failure (short message).
    Error(String),
}

/// True when the app is a real install (NOT launched from `…\target\`).
pub fn is_installed() -> bool {
    std::env::current_exe()
        .ok()
        .map(|p| !p.to_string_lossy().to_lowercase().contains(r"\target\"))
        .unwrap_or(false)
}

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(UA)
        .timeout(std::time::Duration::from_secs(25))
        .build()
        .map_err(|e| e.to_string())
}

/// `(version_without_v, exe_download_url)` of the latest published release.
async fn latest_release() -> Result<(String, String), String> {
    let url = format!("https://api.github.com/repos/{OWNER}/{REPO}/releases/latest");
    let resp = client()?
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("GitHub a renvoyé {}", resp.status()));
    }
    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    let tag = json["tag_name"]
        .as_str()
        .ok_or_else(|| "réponse GitHub inattendue (tag_name absent)".to_string())?;
    let version = tag.trim_start_matches('v').to_string();

    let exe_url = json["assets"]
        .as_array()
        .and_then(|assets| {
            assets
                .iter()
                .find(|a| {
                    a["name"]
                        .as_str()
                        .map(|n| n.to_lowercase().ends_with(".exe"))
                        .unwrap_or(false)
                })
                .and_then(|a| a["browser_download_url"].as_str())
        })
        .ok_or_else(|| "aucun .exe dans la dernière release".to_string())?
        .to_string();

    Ok((version, exe_url))
}

/// CHECK ONLY — used at startup. Inert (and offline) for dev builds.
pub async fn check() -> UpdateState {
    if !is_installed() {
        return UpdateState::Idle;
    }
    match latest_release().await {
        Ok((version, _)) => {
            if is_newer(&version, CURRENT) {
                UpdateState::Available(version)
            } else {
                UpdateState::UpToDate
            }
        }
        Err(e) => UpdateState::Error(short(&e)),
    }
}

/// DOWNLOAD + atomically swap the running exe. When `relaunch`, spawn the new
/// binary before returning `ReadyToQuit` (the UI then exits, freeing the file).
pub async fn apply(relaunch: bool) -> UpdateState {
    if !is_installed() {
        return UpdateState::Idle;
    }
    let url = match latest_release().await {
        Ok((_, url)) => url,
        Err(e) => return UpdateState::Error(short(&e)),
    };
    let bytes = match download(&url).await {
        Ok(b) => b,
        Err(e) => return UpdateState::Error(short(&e)),
    };
    let tmp = std::env::temp_dir().join("onlyclimb-launcher.update.exe");
    if let Err(e) = std::fs::write(&tmp, &bytes) {
        return UpdateState::Error(short(&e.to_string()));
    }
    // Swaps the currently running executable with `tmp` (handles the Windows
    // "can't delete a running exe" dance internally).
    if let Err(e) = self_replace::self_replace(&tmp) {
        return UpdateState::Error(short(&e.to_string()));
    }
    let _ = std::fs::remove_file(&tmp);

    if relaunch {
        if let Ok(exe) = std::env::current_exe() {
            let _ = std::process::Command::new(exe).spawn();
        }
    }
    UpdateState::ReadyToQuit
}

/// Synchronous entry point for the hidden `--self-update` CLI flag (used to test
/// the update pipeline headlessly). Swaps the exe but does not relaunch.
pub fn cli_self_update() -> UpdateState {
    match tokio::runtime::Runtime::new() {
        Ok(rt) => rt.block_on(apply(false)),
        Err(e) => UpdateState::Error(e.to_string()),
    }
}

async fn download(url: &str) -> Result<Vec<u8>, String> {
    let resp = client()?.get(url).send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("téléchargement: {}", resp.status()));
    }
    Ok(resp.bytes().await.map_err(|e| e.to_string())?.to_vec())
}

/// Compares two "x.y.z" versions numerically, field by field.
fn is_newer(remote: &str, current: &str) -> bool {
    fn parts(s: &str) -> Vec<u64> {
        s.split('.')
            .map(|x| x.trim().parse().unwrap_or(0))
            .collect()
    }
    let (r, c) = (parts(remote), parts(current));
    for i in 0..r.len().max(c.len()) {
        let rv = r.get(i).copied().unwrap_or(0);
        let cv = c.get(i).copied().unwrap_or(0);
        if rv != cv {
            return rv > cv;
        }
    }
    false
}

fn short(s: &str) -> String {
    s.lines().next().unwrap_or(s).chars().take(160).collect()
}

#[cfg(test)]
mod tests {
    use super::is_newer;

    #[test]
    fn version_comparison() {
        assert!(is_newer("0.1.1", "0.1.0"));
        assert!(is_newer("0.2.0", "0.1.9"));
        assert!(is_newer("1.0.0", "0.9.9"));
        assert!(!is_newer("0.1.0", "0.1.0"));
        assert!(!is_newer("0.1.0", "0.1.1"));
        assert!(is_newer("0.1.10", "0.1.2")); // numeric, not lexicographic
    }
}
