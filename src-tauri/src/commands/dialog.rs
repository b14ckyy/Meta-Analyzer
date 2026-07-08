// Meta-Analyzer - AI-powered metadata tagger for photos and videos.
// Copyright (C) 2026 b14ckyy
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::collections::HashMap;
use tauri::AppHandle;
use tauri::Emitter;
use tauri_plugin_dialog::{DialogExt, FilePath};
use crate::processing::metadata;
use crate::models::ImportProgressEvent;

#[tauri::command]
pub async fn pick_folder(app: AppHandle) -> Result<Option<String>, String> {
    let (tx, rx) = tokio::sync::oneshot::channel::<Option<FilePath>>();

    app.dialog().file().pick_folder(move |path| {
        let _ = tx.send(path);
    });

    match rx.await {
        Ok(Some(path)) => Ok(Some(path.to_string())),
        Ok(None) => Ok(None),
        Err(_) => Err("Dialog closed unexpectedly".to_string()),
    }
}

/// Öffnet den Datei-Dialog. `kind` steuert den vorausgewählten Filter:
/// "image" → nur Bildformate, "video" → nur Videoformate, sonst kombiniert.
#[tauri::command]
pub async fn pick_files(app: AppHandle, kind: Option<String>) -> Result<Vec<String>, String> {
    let (tx, rx) = tokio::sync::oneshot::channel::<Option<Vec<FilePath>>>();

    let base = app.dialog().file();
    let dialog = match kind.as_deref() {
        Some("image") => base.add_filter("Images", &["jpg", "jpeg", "png"]),
        Some("video") => base.add_filter("Videos", &["mp4", "mkv", "mov", "m4v", "avi", "webm"]),
        _ => base
            .add_filter("Media", &["jpg", "jpeg", "png", "mp4", "mkv", "mov", "m4v", "avi", "webm"])
            .add_filter("Images", &["jpg", "jpeg", "png"])
            .add_filter("Videos", &["mp4", "mkv", "mov", "m4v", "avi", "webm"]),
    };

    dialog.pick_files(move |paths| {
        let _ = tx.send(paths);
    });

    match rx.await {
        Ok(Some(paths)) => Ok(paths
            .into_iter()
            .map(|p| p.to_string())
            .collect()),
        Ok(None) => Ok(vec![]),
        Err(_) => Err("Dialog closed unexpectedly".to_string()),
    }
}

/// Löst per Drag & Drop fallengelassene Pfade auf: Ordner werden nach passenden
/// Medien durchsucht, Einzeldateien nach Endung gefiltert. `kind` = "image" | "video".
#[tauri::command]
pub async fn resolve_media_paths(paths: Vec<String>, kind: String) -> Result<Vec<String>, String> {
    let exts: &[&str] = match kind.as_str() {
        "video" => &["mp4", "mkv", "mov", "m4v", "avi", "webm"],
        _ => &["jpg", "jpeg", "png"],
    };
    let matches_ext = |p: &std::path::Path| -> bool {
        p.extension()
            .and_then(|e| e.to_str())
            .map(|e| exts.contains(&e.to_lowercase().as_str()))
            .unwrap_or(false)
    };

    let mut out: Vec<String> = Vec::new();
    for p in paths {
        let path = std::path::Path::new(&p);
        if path.is_dir() {
            if let Ok(rd) = std::fs::read_dir(path) {
                for entry in rd.flatten() {
                    let ep = entry.path();
                    if ep.is_file() && matches_ext(&ep) {
                        out.push(ep.to_string_lossy().to_string());
                    }
                }
            }
        } else if path.is_file() && matches_ext(path) {
            out.push(p.clone());
        }
    }
    out.sort();
    out.dedup();
    Ok(out)
}

#[tauri::command]
pub async fn scan_folder_for_images(folder: String) -> Result<Vec<String>, String> {
    use std::io;

    let mut paths: Vec<String> = vec![];
    let mut entries: tokio::fs::ReadDir = tokio::fs::read_dir(&folder)
        .await
        .map_err(|e: io::Error| e.to_string())?;

    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|e: io::Error| e.to_string())?
    {
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                let ext = ext.to_lowercase();
                if matches!(ext.as_str(), "jpg" | "jpeg" | "png") {
                    paths.push(path.to_string_lossy().to_string());
                }
            }
        }
    }
    paths.sort();
    Ok(paths)
}

/// Read tags from images, emitting import-progress events with scan results
/// as batches complete. Uses parallel processing for speed.
#[tauri::command]
pub async fn read_images_tags(app: AppHandle, paths: Vec<String>) -> HashMap<String, Vec<String>> {
    let total = paths.len();
    let batch_size = 50; // emit progress every 50 images
    let results = std::sync::Arc::new(std::sync::Mutex::new(HashMap::with_capacity(total)));

    // Process in parallel batches on the blocking thread pool
    let mut handles = Vec::new();
    for chunk in paths.chunks(batch_size) {
        let chunk = chunk.to_vec();
        let results = results.clone();
        let app = app.clone();
        let total = total;

        let handle = tokio::task::spawn_blocking(move || {
            let mut batch_map = HashMap::new();
            for path in &chunk {
                let tags = metadata::read_tags(path);
                batch_map.insert(path.clone(), tags);
            }

            // Merge batch into shared results
            {
                let mut map = results.lock().unwrap();
                map.extend(batch_map);
                let done = map.len();
                // Emit progress
                let _ = app.emit("import-progress", ImportProgressEvent {
                    completed: done,
                    total,
                });
            }
        });
        handles.push(handle);
    }

    // Wait for all batches
    for h in handles {
        let _ = h.await;
    }

    let map = results.lock().unwrap().clone();
    // Final 100% event
    let _ = app.emit("import-progress", ImportProgressEvent {
        completed: total,
        total,
    });

    map
}

/// Original sequential version kept for backward compatibility if needed,
/// but the Tauri command now uses the parallel one above.
#[tauri::command]
pub async fn fetch_available_models(api_url: String, api_key: Option<String>) -> Result<Vec<String>, String> {
    let endpoint = format!("{}/v1/models", api_url.trim_end_matches('/'));
    let client = reqwest::Client::new();

    let mut request = client.get(&endpoint);
    if let Some(key) = api_key.as_deref() {
        if !key.trim().is_empty() {
            request = request.bearer_auth(key.trim());
        }
    }

    match request.send().await {
        Ok(resp) => match resp.json::<serde_json::Value>().await {
            Ok(json) => {
                let models: Vec<String> = json["data"]
                    .as_array()
                    .unwrap_or(&vec![])
                    .iter()
                    .filter_map(|m| m["id"].as_str().map(|s| s.to_string()))
                    .collect();
                Ok(models)
            }
            Err(e) => Err(format!("Failed to parse models response: {}", e)),
        },
        Err(e) => Err(format!("Failed to fetch models (is LM Studio running?): {}", e)),
    }
}
