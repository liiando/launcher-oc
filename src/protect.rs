use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;

const KEY_SEED: &str = "ONLYCLIMB_SECURE_2026";
const EXE_NAME: &str = "system_core.exe";
const BIN_NAME: &str = "system_core.bin";
const TMP_NAME: &str = "system_core.tmp";

fn get_key() -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(KEY_SEED.as_bytes());
    hasher.finalize().to_vec()
}

fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .unwrap_or_else(|_| PathBuf::from("."))
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .to_path_buf()
}

/// Minimum sensible size for an encrypted game binary (4 KiB).
/// If `system_core.bin` is smaller it was corrupted by a failed write on a
/// protected directory — most likely an empty file left by `File::create`
/// when `write_all` was rejected.
const MIN_BIN_SIZE: u64 = 4096;

pub fn encrypt_if_needed() {
    let dir = exe_dir();
    let exe_path = dir.join(EXE_NAME);
    let bin_path = dir.join(BIN_NAME);

    // A valid .bin must be non-trivial. If a previous launch left a 0-byte
    // stub (e.g. `CreateFile` succeeded but `WriteFile` was denied), purge
    // it so we re-encrypt from the real `system_core.exe`.
    if let Ok(meta) = fs::metadata(&bin_path) {
        if meta.len() >= MIN_BIN_SIZE {
            return; // already encrypted successfully
        }
        let _ = fs::remove_file(&bin_path);
    }

    if !exe_path.exists() {
        return;
    }

    let data = fs::read(&exe_path).unwrap_or_default();
    if data.is_empty() {
        return;
    }

    let key = get_key();
    let encrypted: Vec<u8> = data
        .iter()
        .enumerate()
        .map(|(i, b)| b ^ key[i % key.len()])
        .collect();

    if fs::write(&bin_path, &encrypted).is_ok() {
        let _ = fs::remove_file(&exe_path);
    }
}

pub fn decrypt_and_launch() -> Result<(), String> {
    let dir = exe_dir();
    let bin_path = dir.join(BIN_NAME);
    let tmp_path = dir.join(TMP_NAME);

    let data = fs::read(&bin_path).map_err(|e| format!("Lecture du jeu impossible : {e}"))?;
    if data.is_empty() {
        return Err(String::from(
            "Fichier jeu vide (installation corrompue). Relancez le launcher.",
        ));
    }

    let key = get_key();
    let decrypted: Vec<u8> = data
        .iter()
        .enumerate()
        .map(|(i, b)| b ^ key[i % key.len()])
        .collect();

    fs::write(&tmp_path, &decrypted)
        .map_err(|e| format!("Écriture temporaire impossible : {e}"))?;

    match std::process::Command::new(&tmp_path)
        .current_dir(&dir)
        .spawn()
    {
        Ok(_child) => {
            let t = tmp_path.clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(5));
                let _ = fs::remove_file(&t);
            });
            Ok(())
        }
        Err(e) => {
            let _ = fs::remove_file(&tmp_path);
            Err(format!("Impossible de lancer le jeu : {e}"))
        }
    }
}
