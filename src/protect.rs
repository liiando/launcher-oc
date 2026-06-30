use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{AeadCore, Aes256Gcm, Key, Nonce};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;

const EXE_NAME: &str = "system_core.exe";
const BIN_NAME: &str = "system_core.bin";
const TOKEN_NAME: &str = "license.token";
const TMP_PREFIX: &str = "system_core_";
const TMP_SUFFIX: &str = ".tmp";
const MAGIC: &[u8; 4] = b"OCP1";
const SALT: &[u8] = b"ONLYCLIMB_SECURE_2026_SALT";

/// Fingerprint used to encrypt the distributed `system_core.bin`.
/// Must match the key used during deployment, otherwise AES-GCM auth fails.
const MASTER_FINGERPRINT: &str = "D3BC655F35674C56";

/// Minimum sensible size for an AES-GCM encrypted game binary.
/// Magic (4) + nonce (12) + tag (16) + min PE header = at least 8 KiB.
const MIN_BIN_SIZE: u64 = 8192;

fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .unwrap_or_else(|_| PathBuf::from("."))
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .to_path_buf()
}

fn derive_key(fingerprint: &str) -> [u8; 32] {
    let fp_hash = {
        let mut h = Sha256::new();
        h.update(fingerprint.as_bytes());
        h.finalize().to_vec()
    };
    let mut hasher = Sha256::new();
    hasher.update(SALT);
    hasher.update(&fp_hash);
    let result = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result[..32]);
    key
}

pub fn encrypt_if_needed(fingerprint: &str) {
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

    let data = match fs::read(&exe_path) {
        Ok(d) if !d.is_empty() => d,
        _ => return,
    };

    let key = derive_key(fingerprint);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

    let ciphertext = match cipher.encrypt(&nonce, data.as_ref()) {
        Ok(ct) => ct,
        Err(_) => return,
    };

    let mut out = Vec::with_capacity(4 + 12 + ciphertext.len());
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ciphertext);

    if fs::write(&bin_path, &out).is_ok() {
        let _ = fs::remove_file(&exe_path);
    }
}

/// Remove leftover temporary game binaries from previous (possibly crashed)
/// launches so they don't pile up or cause "file in use" errors.
pub fn cleanup_old_temps() {
    let dir = exe_dir();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with(TMP_PREFIX) && name_str.ends_with(TMP_SUFFIX) {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }
}

pub fn generate_token() -> String {
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    hex::encode(nonce.as_slice())
}

pub fn decrypt_and_launch(token: &str) -> Result<(), String> {
    let dir = exe_dir();
    let bin_path = dir.join(BIN_NAME);
    let tmp_name = format!(
        "{}{}{}",
        TMP_PREFIX,
        hex::encode(Aes256Gcm::generate_nonce(&mut OsRng).as_slice()),
        TMP_SUFFIX
    );
    let tmp_path = dir.join(&tmp_name);

    let raw = fs::read(&bin_path).map_err(|e| format!("Lecture du jeu impossible : {e}"))?;

    if raw.len() < 4 + 12 + 16 {
        return Err(String::from(
            "Fichier jeu corrompu (trop petit). Relancez le launcher.",
        ));
    }

    if &raw[..4] != MAGIC {
        return Err(String::from(
            "Fichier jeu invalide (magic OCP1 absent). Reinstallation necessaire.",
        ));
    }

    let nonce = Nonce::from_slice(&raw[4..16]);
    let ciphertext = &raw[16..];

    // Decrypt with the MASTER key that was used to encrypt the distributed .bin
    let key = derive_key(MASTER_FINGERPRINT);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));

    let decrypted = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| String::from("Dechiffrement echoue. Fichier jeu corrompu ou altere."))?;

    fs::write(&tmp_path, &decrypted)
        .map_err(|e| format!("Ecriture temporaire impossible : {e}"))?;

    let token_path = dir.join(TOKEN_NAME);
    fs::write(&token_path, token)
        .map_err(|e| format!("Impossible d'ecrire le jeton de licence : {e}"))?;

    match std::process::Command::new(&tmp_path)
        .current_dir(&dir)
        .env("OC_LICENSE", token)
        .spawn()
    {
        Ok(mut child) => {
            let t = tmp_path.clone();
            let tok = token_path.clone();
            std::thread::spawn(move || {
                // Wait for the game to exit before cleaning up the
                // decrypted temp exe (still in use while running).
                let _ = child.wait();
                let _ = std::fs::remove_file(&t);
                // Token file can be removed earlier, but cleaning
                // everything at once keeps the logic simple.
                let _ = std::fs::remove_file(&tok);
            });
            Ok(())
        }
        Err(e) => {
            let _ = fs::remove_file(&tmp_path);
            let _ = fs::remove_file(&token_path);
            Err(format!("Impossible de lancer le jeu : {e}"))
        }
    }
}
