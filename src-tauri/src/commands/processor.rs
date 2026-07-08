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

use tauri::{AppHandle, State};
use crate::models::{ImageJob, JobStatus, AppSettings, PromptProfile};
use crate::processing::queue::ProcessingQueue;

/// Schreibt die (ggf. editierten) Tags aller DonePending-Foto-Jobs in die Dateien.
/// Wird vom "Apply"-Button aufgerufen, wenn applyAutomatically aus ist.
#[tauri::command]
pub async fn apply_photo_metadata(jobs: Vec<ImageJob>) -> Result<Vec<ImageJob>, String> {
    let mut updated: Vec<ImageJob> = Vec::new();
    for job in jobs {
        if job.status != JobStatus::DonePending {
            updated.push(job);
            continue;
        }
        match crate::processing::metadata::write_metadata(&job.path, &job.tags) {
            Err(e) => {
                let mut j = job;
                j.status = JobStatus::Error;
                j.error_msg = Some(format!("Metadata write failed: {}", e));
                updated.push(j);
            }
            Ok(_) => {
                let mut j = job;
                j.status = JobStatus::Done;
                updated.push(j);
            }
        }
    }
    Ok(updated)
}

#[tauri::command]
pub async fn start_processing(
    app: AppHandle,
    jobs: Vec<ImageJob>,
    settings: AppSettings,
    profile: PromptProfile,
    queue: State<'_, ProcessingQueue>,
) -> Result<(), String> {
    queue
        .start(app, jobs, settings, profile)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn pause_processing(queue: State<'_, ProcessingQueue>) -> Result<(), String> {
    queue.pause().await;
    Ok(())
}

#[tauri::command]
pub async fn resume_processing(queue: State<'_, ProcessingQueue>) -> Result<(), String> {
    queue.resume().await;
    Ok(())
}

#[tauri::command]
pub async fn stop_processing(queue: State<'_, ProcessingQueue>) -> Result<(), String> {
    queue.stop().await;
    Ok(())
}
