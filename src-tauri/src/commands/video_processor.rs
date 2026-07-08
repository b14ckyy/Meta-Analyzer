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

#[tauri::command]
pub async fn apply_video_metadata(
    app: AppHandle,
    jobs: Vec<VideoJob>,
    write_description: bool,
    write_genres: bool,
    write_tags: bool,
    write_title: bool,
) -> Result<Vec<VideoJob>, String> {
    eprintln!("[video_processor] apply_video_metadata: {} jobs, desc={}, genres={}, tags={}, title={}",
        jobs.len(), write_description, write_genres, write_tags, write_title);

    let mut updated_jobs: Vec<VideoJob> = Vec::new();

    for job in jobs {
        if job.status != VideoJobStatus::DonePending {
            updated_jobs.push(job);
            continue;
        }

        // Strukturierte Ergebnisse aus dem Job rekonstruieren. Die Felder überleben
        // seit VideoJob description/genres/title trägt den Round-Trip vom Frontend.
        // keywords = job.tags (dort landen die geparsten Keywords bzw. der Comma-Fallback).
        let meta = crate::video_models::VideoMetaOutput {
            description: job.description.clone(),
            genres: job.genres.clone(),
            keywords: job.tags.clone(),
            title: job.title.clone(),
        };
        match crate::processing::video_metadata::write_video_metadata(
            &job.path,
            &meta,
            write_description,
            write_genres,
            write_tags,
            write_title,
        ) {
            Err(e) => {
                eprintln!("[video_processor] Metadata write failed for {}: {}", job.file_name, e);
                let mut j = job;
                j.status = VideoJobStatus::Error;
                j.error_msg = Some(format!("Metadata write failed: {}", e));
                updated_jobs.push(j);
            }
            Ok(_) => {
                let mut j = job;
                j.status = VideoJobStatus::Done;
                updated_jobs.push(j);
            }
        }
    }

    Ok(updated_jobs)
}

// Original import — wird von den anderen commands benötigt
use tauri::{AppHandle, State};
use crate::models::AppSettings;
use crate::video_models::{VideoJob, VideoSettings, VideoJobStatus, VideoProfile};
use crate::processing::video_queue::VideoProcessingQueue;

#[tauri::command]
pub async fn start_video_processing(
    app: AppHandle,
    queue: State<'_, VideoProcessingQueue>,
    jobs: Vec<VideoJob>,
    settings: AppSettings,
    profile: VideoProfile,
    video_settings: VideoSettings,
) -> Result<(), String> {
    eprintln!("[video_processor] start_video_processing: {} jobs", jobs.len());

    // Prüfe ob es überhaupt pending Jobs gibt
    let has_pending = jobs.iter().any(|j| j.status == VideoJobStatus::Pending);
    if !has_pending {
        return Err("No pending video jobs to process".to_string());
    }

    queue.start(app, jobs, settings, profile, video_settings)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn stop_video_processing(
    queue: State<'_, VideoProcessingQueue>,
) -> Result<(), String> {
    queue.stop().await;
    Ok(())
}

#[tauri::command]
pub async fn pause_video_processing(
    queue: State<'_, VideoProcessingQueue>,
) -> Result<(), String> {
    queue.pause().await;
    Ok(())
}

#[tauri::command]
pub async fn resume_video_processing(
    queue: State<'_, VideoProcessingQueue>,
) -> Result<(), String> {
    queue.resume().await;
    Ok(())
}

#[tauri::command]
pub async fn scan_folder_for_videos(folder: String) -> Result<Vec<String>, String> {
    let mut paths: Vec<String> = vec![];
    let mut entries = tokio::fs::read_dir(&folder)
        .await
        .map_err(|e| e.to_string())?;

    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|e| e.to_string())?
    {
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                let ext = ext.to_lowercase();
                // av1 ist ein Codec, kein Container — daher keine gültige Datei-Endung.
                if matches!(ext.as_str(), "mp4" | "mkv" | "mov" | "m4v" | "avi" | "webm") {
                    paths.push(path.to_string_lossy().to_string());
                }
            }
        }
    }
    paths.sort();
    Ok(paths)
}

#[tauri::command]
pub async fn get_video_duration(video_path: String) -> Result<f64, String> {
    crate::processing::video_decoder::get_duration(&video_path)
        .map_err(|e| e.to_string())
}

/// Sammelt vorhandene .jpg-Thumbnails aus einem Ordner (sortiert).
fn read_thumb_dir(dir: &std::path::Path) -> Vec<String> {
    let mut v = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|x| x.to_str()) == Some("jpg") {
                v.push(p.to_string_lossy().to_string());
            }
        }
    }
    v.sort();
    v
}

/// Erzeuge Vorschau-Thumbnails für ein Video (falls noch nicht vorhanden) und gib deren
/// Pfade zurück. Bevorzugt ein eingebettetes Cover; sonst 5 gleichmäßig verteilte Frames.
/// Wird beim Import aufgerufen, damit die Liste eine Vorschau zeigt, bevor die volle
/// Frame-Extraktion läuft. Ergebnisse werden pro Video zwischengespeichert.
#[tauri::command]
pub async fn create_video_thumbnail(video_path: String) -> Result<Vec<String>, String> {
    use std::hash::{Hash, Hasher};

    let base = std::env::temp_dir().join("meta-analyzer").join("thumbs");
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    video_path.hash(&mut hasher);
    let dir = base.join(format!("{:x}", hasher.finish()));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    // Cache: bereits erzeugte Thumbnails wiederverwenden.
    let existing = read_thumb_dir(&dir);
    if !existing.is_empty() {
        return Ok(existing);
    }

    // 1) Eingebettetes Cover bevorzugen (falls vorhanden).
    let embedded = dir.join("thumb_embedded.jpg");
    if let Ok(true) = crate::processing::video_decoder::extract_embedded_thumbnail(&video_path, &embedded) {
        return Ok(vec![embedded.to_string_lossy().to_string()]);
    }

    // 2) Sonst 5 gleichmäßig verteilte Frames.
    crate::processing::video_decoder::extract_thumbnails(&video_path, &dir, 5, 480, 270)
        .map(|paths| paths.iter().map(|p| p.to_string_lossy().to_string()).collect())
        .map_err(|e| e.to_string())
}

/// Erstelle einen VideoJob und ermittle direkt die Dauer via ffprobe
#[tauri::command]
pub async fn create_video_job(video_path: String) -> Result<VideoJob, String> {
    let mut job = VideoJob::new(video_path);
    // Dauer ermitteln
    match crate::processing::video_decoder::get_duration(&job.path) {
        Ok(duration) => job.duration_secs = duration,
        Err(e) => {
            eprintln!("[video_processor] Could not get duration for {}: {}", job.file_name, e);
            // Nicht fatal — wir haben zumindest den Job
        }
    }
    Ok(job)
}
