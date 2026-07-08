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

use std::io;
use tauri::AppHandle;
use crate::models::{AppSettings, PromptProfile};
use crate::video_models::VideoProfile;

// ── App Settings (unchanged logic) ───────────────────────────────────────

#[tauri::command]
pub async fn load_settings(app: AppHandle) -> Result<AppSettings, String> {
    let app_data_dir = crate::paths::data_dir(&app)
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    let settings_path = app_data_dir.join("settings.json");

    if settings_path.exists() {
        let text = tokio::fs::read_to_string(&settings_path)
            .await
            .map_err(|e: io::Error| e.to_string())?;
        serde_json::from_str(&text).map_err(|e| e.to_string())
    } else {
        Ok(AppSettings::default())
    }
}

#[tauri::command]
pub async fn save_settings(app: AppHandle, settings: AppSettings) -> Result<(), String> {
    let app_data_dir = crate::paths::data_dir(&app)
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    tokio::fs::create_dir_all(&app_data_dir)
        .await
        .map_err(|e: io::Error| e.to_string())?;

    let text = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    let settings_path = app_data_dir.join("settings.json");

    #[allow(clippy::type_complexity)]
    let result: Result<(), io::Error> = tokio::fs::write(&settings_path, &text).await;
    result.map_err(|e| e.to_string())
}

// ── Prompt Profiles CRUD ─────────────────────────────────────────────────

fn profiles_dir(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = crate::paths::data_dir(app)
        .map_err(|e| format!("App data dir: {}", e))?
        .join("profiles");
    std::fs::create_dir_all(&dir).map_err(|e: io::Error| e.to_string())?;
    Ok(dir)
}

/// List all saved profile names.
#[tauri::command]
pub async fn list_profiles(app: AppHandle) -> Result<Vec<String>, String> {
    let dir = profiles_dir(&app)?;
    let mut names: Vec<String> = Vec::new();

    let mut entries = tokio::fs::read_dir(&dir)
        .await
        .map_err(|e: io::Error| e.to_string())?;

    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|e: io::Error| e.to_string())?
    {
        if entry.path().extension().map(|e| e == "json").unwrap_or(false) {
            if let Some(stem) = entry.path().file_stem().and_then(|s| s.to_str()) {
                names.push(stem.to_string());
            }
        }
    }

    names.sort();
    Ok(names)
}

/// Load a single profile by name. Returns default profile if not found.
#[tauri::command]
pub async fn load_profile(app: AppHandle, name: String) -> Result<PromptProfile, String> {
    let dir = profiles_dir(&app)?;
    let path = dir.join(format!("{}.json", name));

    if path.exists() {
        let text = tokio::fs::read_to_string(&path)
            .await
            .map_err(|e: io::Error| e.to_string())?;
        serde_json::from_str(&text).map_err(|e| e.to_string())
    } else {
        // Return default profile with requested name
        let mut p = PromptProfile::default();
        p.name = name;
        Ok(p)
    }
}

/// Save (create or overwrite) a profile.
#[tauri::command]
pub async fn save_profile(app: AppHandle, profile: PromptProfile) -> Result<(), String> {
    let dir = profiles_dir(&app)?;
    let path = dir.join(format!("{}.json", profile.name));
    let text = serde_json::to_string_pretty(&profile).map_err(|e| e.to_string())?;
    tokio::fs::write(&path, &text)
        .await
        .map_err(|e: io::Error| e.to_string())
}

/// Delete a profile by name.
#[tauri::command]
pub async fn delete_profile(app: AppHandle, name: String) -> Result<(), String> {
    let dir = profiles_dir(&app)?;
    let path = dir.join(format!("{}.json", name));
    if path.exists() {
        tokio::fs::remove_file(&path)
            .await
            .map_err(|e: io::Error| e.to_string())
    } else {
        Ok(())
    }
}

/// Load vocabulary for a given content type label.
#[tauri::command]
pub async fn load_vocabulary(app: AppHandle, content_type: String) -> Result<Vec<String>, String> {
    let app_data_dir = crate::paths::data_dir(&app)
        .map_err(|e| format!("App data dir: {}", e))?;
    let slug = crate::content_types::photo_slug(&app_data_dir, &content_type);
    Ok(crate::processing::prompt_builder::load_vocabulary(&app_data_dir, &slug))
}

/// Generate a prompt preview exactly as it would be sent to the LLM.
/// Uses the same `build_prompt()` function that queue.rs uses.
#[tauri::command]
pub async fn preview_prompt(app: AppHandle, profile: PromptProfile) -> Result<String, String> {
    use crate::processing::prompt_builder;

    let app_data_dir = crate::paths::data_dir(&app)
        .map_err(|e| format!("App data dir: {}", e))?;

    let slug = crate::content_types::photo_slug(&app_data_dir, &profile.content_type);
    let vocabulary = prompt_builder::load_vocabulary(&app_data_dir, &slug);
    let content_rules = prompt_builder::load_content_rules(&app_data_dir, &slug);
    let prompt = prompt_builder::build_prompt(&profile, &vocabulary, &content_rules, false);
    Ok(prompt)
}

// ── Video Profiles CRUD ──────────────────────────────────────────────────
// Selbe Logik wie Prompt Profiles, aber im Unterordner "video_profiles/".
// VideoProfile hat eigene Content-Types und eingebautes custom_prompt.

fn video_profiles_dir(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = crate::paths::data_dir(app)
        .map_err(|e| format!("App data dir: {}", e))?
        .join("video_profiles");
    std::fs::create_dir_all(&dir).map_err(|e: io::Error| e.to_string())?;
    Ok(dir)
}

#[tauri::command]
pub async fn list_video_profiles(app: AppHandle) -> Result<Vec<String>, String> {
    let dir = video_profiles_dir(&app)?;
    let mut names: Vec<String> = Vec::new();

    let mut entries = tokio::fs::read_dir(&dir)
        .await
        .map_err(|e: io::Error| e.to_string())?;

    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|e: io::Error| e.to_string())?
    {
        if entry.path().extension().map(|e| e == "json").unwrap_or(false) {
            if let Some(stem) = entry.path().file_stem().and_then(|s| s.to_str()) {
                names.push(stem.to_string());
            }
        }
    }

    names.sort();
    Ok(names)
}

#[tauri::command]
pub async fn load_video_profile(app: AppHandle, name: String) -> Result<VideoProfile, String> {
    let dir = video_profiles_dir(&app)?;
    let path = dir.join(format!("{}.json", name));

    if path.exists() {
        let text = tokio::fs::read_to_string(&path)
            .await
            .map_err(|e: io::Error| e.to_string())?;
        serde_json::from_str(&text).map_err(|e| e.to_string())
    } else {
        let mut p = VideoProfile::default();
        p.name = name;
        Ok(p)
    }
}

#[tauri::command]
pub async fn save_video_profile(app: AppHandle, profile: VideoProfile) -> Result<(), String> {
    let dir = video_profiles_dir(&app)?;
    let path = dir.join(format!("{}.json", profile.name));
    let text = serde_json::to_string_pretty(&profile).map_err(|e| e.to_string())?;
    tokio::fs::write(&path, &text)
        .await
        .map_err(|e: io::Error| e.to_string())
}

#[tauri::command]
pub async fn delete_video_profile(app: AppHandle, name: String) -> Result<(), String> {
    let dir = video_profiles_dir(&app)?;
    let path = dir.join(format!("{}.json", name));
    if path.exists() {
        tokio::fs::remove_file(&path)
            .await
            .map_err(|e: io::Error| e.to_string())
    } else {
        Ok(())
    }
}

/// List available photo content types: curated defaults + any rule files the
/// user dropped into {app_data}/content_rules/ (including optional add-on packs).
#[tauri::command]
pub async fn list_content_types(app: AppHandle) -> Result<Vec<crate::content_types::ContentType>, String> {
    let app_data_dir = crate::paths::data_dir(&app)
        .map_err(|e| format!("App data dir: {}", e))?;
    Ok(crate::content_types::list_photo_content_types(&app_data_dir))
}

/// List available video content types: curated defaults + discovered extras.
#[tauri::command]
pub async fn list_video_content_types(app: AppHandle) -> Result<Vec<crate::content_types::ContentType>, String> {
    let app_data_dir = crate::paths::data_dir(&app)
        .map_err(|e| format!("App data dir: {}", e))?;
    Ok(crate::content_types::list_video_content_types(&app_data_dir))
}

/// Load video vocabulary for a given video content type label.
#[tauri::command]
pub async fn load_video_vocabulary(app: AppHandle, video_content_type: String) -> Result<Vec<String>, String> {
    let app_data_dir = crate::paths::data_dir(&app)
        .map_err(|e| format!("App data dir: {}", e))?;
    let slug = crate::content_types::video_slug(&app_data_dir, &video_content_type);
    Ok(crate::processing::video_prompt_builder::load_vocabulary(&app_data_dir, &slug))
}

/// Generate a video prompt preview using the new video_prompt_builder.
#[tauri::command]
pub async fn preview_video_prompt(app: AppHandle, profile: VideoProfile) -> Result<String, String> {
    use crate::processing::video_prompt_builder;

    let app_data_dir = crate::paths::data_dir(&app)
        .map_err(|e| format!("App data dir: {}", e))?;

    let slug = crate::content_types::video_slug(&app_data_dir, &profile.content_type);
    let vocabulary = video_prompt_builder::load_vocabulary(&app_data_dir, &slug);
    let content_rules = video_prompt_builder::load_video_content_rules(&app_data_dir, &slug);
    let prompt = video_prompt_builder::build_video_prompt(&profile, &vocabulary, &content_rules);
    Ok(prompt)
}

/// Öffnet eine Datei mit dem system-default Programm (via cmd /c start).
/// Wird für "Show Reasoning" in der UI verwendet.
#[tauri::command]
pub async fn open_file(path: String) -> Result<(), String> {
    std::process::Command::new("cmd")
        .args(["/c", "start", "", &path])
        .spawn()
        .map_err(|e| format!("Failed to open file: {}", e))?;
    Ok(())
}

/// Öffnet den Windows-Explorer und markiert die Datei.
#[tauri::command]
pub async fn reveal_in_explorer(path: String) -> Result<(), String> {
    std::process::Command::new("explorer")
        .arg(format!("/select,{}", path))
        .spawn()
        .map_err(|e| format!("Failed to reveal file: {}", e))?;
    Ok(())
}

/// Opens the editable content_rules folder (creating it if needed) in Explorer,
/// so users can add/edit rule files and drop in packs.
#[tauri::command]
pub async fn open_content_rules_dir(app: AppHandle) -> Result<(), String> {
    let dir = crate::paths::data_dir(&app)
        .map_err(|e| format!("App data dir: {}", e))?
        .join("content_rules");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    std::process::Command::new("explorer")
        .arg(&dir)
        .spawn()
        .map_err(|e| format!("Failed to open folder: {}", e))?;
    Ok(())
}

