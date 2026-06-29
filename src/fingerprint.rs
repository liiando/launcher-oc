use sha2::{Digest, Sha256};
use std::process::Command;

pub fn generate_fingerprint() -> String {
    let uuid = run_wmic("csproduct", "uuid");
    let vol = run_wmic(r#"logicaldisk where name="C:""#, "volumeserialnumber");
    let guid = read_registry_guid();

    let raw = format!("{}|{}|{}", uuid.trim(), vol.trim(), guid.trim());
    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    hex::encode(hasher.finalize())[..16].to_uppercase()
}

/// Builds a `Command` that never flashes a console window. `wmic`/`reg` are
/// console programs, so without `CREATE_NO_WINDOW` each call pops a terminal
/// even when the launcher itself is a GUI app.
fn hidden_command(program: &str) -> Command {
    let mut cmd = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

fn run_wmic(class: &str, property: &str) -> String {
    let output = hidden_command("wmic")
        .args([class, "get", property])
        .output();

    match output {
        Ok(out) => {
            let text = String::from_utf8_lossy(&out.stdout);
            text.lines().nth(1).unwrap_or("UNKNOWN").trim().to_string()
        }
        Err(_) => "UNKNOWN".to_string(),
    }
}

fn read_registry_guid() -> String {
    let output = hidden_command("reg")
        .args([
            "query",
            r"HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Cryptography",
            "/v",
            "MachineGuid",
        ])
        .output();

    match output {
        Ok(out) => {
            let text = String::from_utf8_lossy(&out.stdout);
            for line in text.lines() {
                if line.contains("MachineGuid") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    return parts.last().unwrap_or(&"UNKNOWN").to_string();
                }
            }
            "UNKNOWN".to_string()
        }
        Err(_) => "UNKNOWN".to_string(),
    }
}
