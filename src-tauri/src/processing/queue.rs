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

use std::sync::Arc;
use tokio::sync::{Mutex, Notify};
use tokio_util::sync::CancellationToken;
use tauri::{AppHandle, Emitter};
use crate::models::*;
use super::pool::{self, JobOutcome, PauseControl, PoolConfig, PoolJob};
use super::{ai_client, metadata, prompt_builder};

impl PoolJob for ImageJob {
    fn is_pending(&self) -> bool {
        self.status == JobStatus::Pending
    }
    fn reset_to_pending(&mut self) {
        self.status = JobStatus::Pending;
    }
}

pub struct ProcessingQueue {
    cancel_token: Arc<Mutex<Option<CancellationToken>>>,
    pause_notify: Arc<Notify>,
    is_paused: Arc<Mutex<bool>>,
}

impl ProcessingQueue {
    pub fn new() -> Self {
        Self {
            cancel_token: Arc::new(Mutex::new(None)),
            pause_notify: Arc::new(Notify::new()),
            is_paused: Arc::new(Mutex::new(false)),
        }
    }

    pub async fn start(
        &self,
        app: AppHandle,
        jobs: Vec<ImageJob>,
        settings: AppSettings,
        profile: PromptProfile,
    ) -> anyhow::Result<()> {
        self.stop().await;

        let token = CancellationToken::new();
        *self.cancel_token.lock().await = Some(token.clone());

        let pause = PauseControl {
            is_paused: self.is_paused.clone(),
            notify: self.pause_notify.clone(),
        };

        let max_concurrent = settings.max_concurrent.clamp(1, 8) as usize;
        let total = jobs.len();
        eprintln!("[queue] Starting with max_concurrent={}, total_jobs={}", max_concurrent, total);

        tokio::spawn(async move {
            let client = reqwest::Client::new();

            // Prompt einmal bauen (für alle Worker geteilt).
            let app_data_dir = crate::paths::data_dir(&app).unwrap_or_default();
            let slug = crate::content_types::photo_slug(&app_data_dir, &profile.content_type);
            let vocabulary = prompt_builder::load_vocabulary(&app_data_dir, &slug);
            let content_rules = prompt_builder::load_content_rules(&app_data_dir, &slug);
            let prompt = prompt_builder::build_prompt(&profile, &vocabulary, &content_rules, false);

            let config = PoolConfig {
                max_concurrent,
                total,
                progress_event: "progress",
                log_tag: "queue",
            };

            // Klone für den Pool-Aufruf selbst (die process_one-Closure konsumiert die anderen).
            let app_pool = app.clone();
            let token_pool = token.clone();

            let process_one = move |job: ImageJob| {
                let client = client.clone();
                let settings = settings.clone();
                let app = app.clone();
                let token = token.clone();
                let prompt = prompt.clone();
                async move { process_photo_job(job, &client, &settings, &app, &token, &prompt).await }
            };

            pool::run_worker_pool(app_pool, jobs, token_pool, Some(pause), config, process_one).await;
        });

        Ok(())
    }

    pub async fn pause(&self) {
        *self.is_paused.lock().await = true;
    }

    pub async fn resume(&self) {
        *self.is_paused.lock().await = false;
        self.pause_notify.notify_waiters();
    }

    pub async fn stop(&self) {
        if let Some(token) = self.cancel_token.lock().await.take() {
            token.cancel();
        }
        *self.is_paused.lock().await = false;
        self.pause_notify.notify_waiters();
    }
}

/// Verarbeitet einen einzelnen Foto-Job: Analyse (mit 1 Retry) → Metadaten schreiben → Events.
async fn process_photo_job(
    mut job: ImageJob,
    client: &reqwest::Client,
    settings: &AppSettings,
    app: &AppHandle,
    token: &CancellationToken,
    prompt: &str,
) -> (ImageJob, JobOutcome) {
    eprintln!("[queue] → Processing job {} ({})", job.id, job.file_name);
    let _ = app.emit("job-update", JobUpdateEvent {
        job_id: job.id.clone(),
        status: JobStatus::Processing,
        tags: vec![],
        error_msg: None,
    });

    // Analyse mit einem Retry — außer der Fehler kam durch STOP.
    let result = ai_client::analyze_image(client, settings, &job.path, app, &job.id, prompt, Some(token.clone())).await;
    let result = match result {
        Ok(tags) => Ok(tags),
        Err(first_err) => {
            if token.is_cancelled() {
                Err(first_err)
            } else {
                eprintln!("[queue] First attempt failed ({}), retrying…", first_err);
                ai_client::analyze_image(client, settings, &job.path, app, &job.id, prompt, Some(token.clone())).await
            }
        }
    };

    let (status, tags, error_msg) = match result {
        Ok(tags) => {
            if settings.apply_automatically {
                match metadata::write_metadata(&job.path, &tags) {
                    Err(e) => (JobStatus::Error, tags, Some(format!("Metadata write failed: {}", e))),
                    Ok(_) => (JobStatus::Done, tags, None),
                }
            } else {
                // Auto-Apply aus → auf manuelles Apply warten (Tags sind editierbar).
                (JobStatus::DonePending, tags, None)
            }
        }
        Err(_e) if token.is_cancelled() => {
            eprintln!("[queue] Cancelled job {} — resetting to pending", job.file_name);
            let _ = app.emit("job-update", JobUpdateEvent {
                job_id: job.id.clone(),
                status: JobStatus::Pending,
                tags: vec![],
                error_msg: None,
            });
            return (job, JobOutcome::Cancelled);
        }
        Err(e) => (JobStatus::Error, vec![], Some(e.to_string())),
    };

    job.tags = tags;
    let _ = app.emit("job-update", JobUpdateEvent {
        job_id: job.id.clone(),
        status,
        tags: job.tags.clone(),
        error_msg,
    });

    (job, JobOutcome::Done)
}
