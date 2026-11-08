use base64::{engine::general_purpose::STANDARD, Engine as _};
use minisign_verify::{PublicKey, Signature};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Manager};

/// Chiave pubblica (base64 del file .pub di minisign) usata per verificare le patch.
const PUBKEY_B64: &str = include_str!("../pubkey.txt");
const BACKUP_GIORNALIERI: usize = 14;

fn dir_dati(app: &AppHandle) -> Result<PathBuf, String> {
    let d = app.path().app_data_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&d).map_err(|e| format!("Impossibile creare la cartella dei dati: {e}"))?;
    Ok(d)
}

fn secondi() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Data odierna (UTC) nel formato AAAA-MM-GG, senza dipendenze esterne.
fn data_oggi() -> String {
    let z = secondi().div_euclid(86400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{:04}-{:02}-{:02}", y, m, d)
}

/// Una copia al giorno dello stato precedente, ne tiene le ultime 14.
fn backup_giornaliero(d: &PathBuf) {
    let attuale = d.join("dati.json");
    if !attuale.exists() {
        return;
    }
    let oggi = d.join(format!("dati.backup-{}.json", data_oggi()));
    if !oggi.exists() {
        let _ = fs::copy(&attuale, &oggi);
    }
    if let Ok(rd) = fs::read_dir(d) {
        let mut nomi: Vec<String> = rd
            .filter_map(|e| e.ok())
            .filter_map(|e| e.file_name().into_string().ok())
            .filter(|n| n.starts_with("dati.backup-") && n.ends_with(".json"))
            .collect();
        nomi.sort();
        while nomi.len() > BACKUP_GIORNALIERI {
            let vecchio = nomi.remove(0);
            let _ = fs::remove_file(d.join(vecchio));
        }
    }
}

#[tauri::command]
fn cartella_dati(app: AppHandle) -> Result<String, String> {
    Ok(dir_dati(&app)?.to_string_lossy().into_owned())
}

#[tauri::command]
fn leggi_dati(app: AppHandle) -> Result<Option<String>, String> {
    let p = dir_dati(&app)?.join("dati.json");
    if !p.exists() {
        return Ok(None);
    }
    fs::read_to_string(&p)
        .map(Some)
        .map_err(|e| format!("Lettura dei dati non riuscita: {e}"))
}

/// Scrittura atomica: prima su file temporaneo, poi sostituzione.
#[tauri::command]
fn scrivi_dati(app: AppHandle, contenuto: String) -> Result<(), String> {
    let d = dir_dati(&app)?;
    backup_giornaliero(&d);
    let tmp = d.join("dati.json.tmp");
    fs::write(&tmp, contenuto.as_bytes()).map_err(|e| format!("Scrittura non riuscita: {e}"))?;
    fs::rename(&tmp, d.join("dati.json")).map_err(|e| format!("Salvataggio non riuscito: {e}"))
}

/// Copia di sicurezza che non viene mai cancellata da sola (prima di un'importazione).
#[tauri::command]
fn copia_sicurezza(app: AppHandle) -> Result<Option<String>, String> {
    let d = dir_dati(&app)?;
    let attuale = d.join("dati.json");
    if !attuale.exists() {
        return Ok(None);
    }
    let nome = format!("dati.prima-importazione-{}.json", secondi());
    fs::copy(&attuale, d.join(&nome)).map_err(|e| format!("Copia di sicurezza non riuscita: {e}"))?;
    Ok(Some(nome))
}

#[tauri::command]
fn esporta_file(app: AppHandle, filename: String, contenuto: String) -> Result<String, String> {
    let nome: String = filename
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' { c } else { '_' })
        .collect();
    if nome.is_empty() || nome.starts_with('.') {
        return Err("Nome file non valido".into());
    }
    let base = app
        .path()
        .download_dir()
        .or_else(|_| app.path().document_dir())
        .map_err(|e| e.to_string())?;
    let p = base.join(&nome);
    fs::write(&p, contenuto.as_bytes()).map_err(|e| format!("Salvataggio non riuscito: {e}"))?;
    Ok(p.to_string_lossy().into_owned())
}

// ---------------------------------------------------------------- patch

#[derive(Serialize)]
struct Patch {
    versione: String,
    html: String,
}

#[derive(Deserialize)]
struct FilePatch {
    formato: String,
    versione: String,
    html_b64: String,
    firma: String,
}

fn versione_valida(v: &str) -> bool {
    let parti: Vec<&str> = v.split('.').collect();
    parti.len() == 3 && parti.iter().all(|p| !p.is_empty() && p.len() <= 6 && p.chars().all(|c| c.is_ascii_digit()))
}

fn verifica(dati: &[u8], firma_b64: &str) -> Result<(), String> {
    let pk_txt = String::from_utf8(
        STANDARD
            .decode(PUBKEY_B64.trim())
            .map_err(|_| "Chiave pubblica non valida".to_string())?,
    )
    .map_err(|_| "Chiave pubblica non valida".to_string())?;
    let pk = PublicKey::decode(&pk_txt).map_err(|e| format!("Chiave pubblica non valida: {e}"))?;
    let sig_txt = String::from_utf8(
        STANDARD
            .decode(firma_b64.trim())
            .map_err(|_| "Firma illeggibile".to_string())?,
    )
    .map_err(|_| "Firma illeggibile".to_string())?;
    let sig = Signature::decode(&sig_txt).map_err(|_| "Firma illeggibile".to_string())?;
    pk.verify(dati, &sig, true)
        .map_err(|_| "Firma non valida: la patch non è autentica".to_string())
}

#[tauri::command]
fn installa_patch(app: AppHandle, contenuto: String) -> Result<String, String> {
    let f: FilePatch =
        serde_json::from_str(&contenuto).map_err(|_| "Il file non è una patch di Crediti Service".to_string())?;
    if f.formato != "crediti-service-patch" || !versione_valida(&f.versione) {
        return Err("Il file non è una patch di Crediti Service".into());
    }
    let html = STANDARD
        .decode(f.html_b64.trim())
        .map_err(|_| "Contenuto della patch illeggibile".to_string())?;
    verifica(&html, &f.firma)?;
    let p = dir_dati(&app)?.join("patch");
    fs::create_dir_all(&p).map_err(|e| e.to_string())?;
    fs::write(p.join("index.html"), &html).map_err(|e| e.to_string())?;
    fs::write(p.join("firma.txt"), f.firma.trim()).map_err(|e| e.to_string())?;
    fs::write(p.join("versione.txt"), &f.versione).map_err(|e| e.to_string())?;
    Ok(f.versione)
}

#[tauri::command]
fn leggi_patch(app: AppHandle) -> Result<Option<Patch>, String> {
    let p = dir_dati(&app)?.join("patch");
    let (Ok(html), Ok(firma), Ok(versione)) = (
        fs::read(p.join("index.html")),
        fs::read_to_string(p.join("firma.txt")),
        fs::read_to_string(p.join("versione.txt")),
    ) else {
        return Ok(None);
    };
    let versione = versione.trim().to_string();
    // la firma viene ricontrollata a ogni avvio
    if !versione_valida(&versione) || verifica(&html, &firma).is_err() {
        return Ok(None);
    }
    match String::from_utf8(html) {
        Ok(html) => Ok(Some(Patch { versione, html })),
        Err(_) => Ok(None),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            cartella_dati,
            leggi_dati,
            scrivi_dati,
            copia_sicurezza,
            esporta_file,
            installa_patch,
            leggi_patch
        ])
        .run(tauri::generate_context!())
        .expect("errore nell'avvio di Crediti Service");
}
