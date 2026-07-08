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
use crate::video_models::*;
use crate::models::AppSettings;
use super::pool::{self, JobOutcome, PauseControl, PoolConfig, PoolJob};
use super::{ai_client, video_decoder, video_prompt_builder, video_metadata};

impl PoolJob for VideoJob {
    fn is_pending(&self) -> bool {
        self.status == VideoJobStatus::Pending
    }
    fn reset_to_pending(&mut self) {
        self.status = VideoJobStatus::Pending;
    }
}

/// Bereinige temporäre Verzeichnisse aus vorherigen Sitzungen.
/// Wird beim Start einmal aufgerufen (in main.rs).
pub fn cleanup_temp_dirs() {
    let temp_base = std::env::temp_dir().join("meta-analyzer");
    if temp_base.exists() {
        let _ = std::fs::remove_dir_all(&temp_base);
        eprintln!("[video_queue] Cleaned up temp dir: {:?}", temp_base);
    }
}

pub struct VideoProcessingQueue {
    cancel_token: Arc<Mutex<Option<CancellationToken>>>,
    pause_notify: Arc<Notify>,
    is_paused: Arc<Mutex<bool>>,
}

impl VideoProcessingQueue {
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
        jobs: Vec<VideoJob>,
        settings: AppSettings,
        profile: VideoProfile,
        video_settings: VideoSettings,
    ) -> anyhow::Result<()> {
        self.stop().await;

        // Prüfe ob ffmpeg/ffprobe verfügbar (App-Verzeichnis → PATH)
        if let Some(path) = video_decoder::ffmpeg_path() {
            eprintln!("[video_queue] ffmpeg found at: {:?}", path);
        } else {
            return Err(anyhow::anyhow!(
                "ffmpeg not found. Place ffmpeg.exe next to Meta-Analyzer.exe, or add it to your PATH."
            ));
        }
        if let Some(path) = video_decoder::ffprobe_path() {
            eprintln!("[video_queue] ffprobe found at: {:?}", path);
        } else {
            return Err(anyhow::anyhow!(
                "ffprobe not found. Place ffprobe.exe next to Meta-Analyzer.exe, or add it to your PATH."
            ));
        }

        let token = CancellationToken::new();
        *self.cancel_token.lock().await = Some(token.clone());

        // Pause zurücksetzen (falls von einem vorherigen Lauf noch gesetzt).
        *self.is_paused.lock().await = false;
        let pause = PauseControl {
            is_paused: self.is_paused.clone(),
            notify: self.pause_notify.clone(),
        };

        let max_concurrent = (video_settings.max_concurrent as usize).clamp(1, 4);
        let total = jobs.len();
        eprintln!("[video_queue] Starting with max_concurrent={}, total_jobs={}", max_concurrent, total);

        tokio::spawn(async move {
            let client = reqwest::Client::new();

            // Prompt 1× bauen — mit eigenem video_prompt_builder.
            let app_data_dir = crate::paths::data_dir(&app).unwrap_or_default();
            let slug = crate::content_types::video_slug(&app_data_dir, &profile.content_type);
            let vocabulary = video_prompt_builder::load_vocabulary(&app_data_dir, &slug);
            let content_rules = video_prompt_builder::load_video_content_rules(&app_data_dir, &slug);
            let prompt = video_prompt_builder::build_video_prompt(&profile, &vocabulary, &content_rules);

            // Custom Prompt aus dem Profile einbauen (falls vorhanden).
            let video_prompt = if !profile.custom_prompt.is_empty() {
                format!("{}\n\n{}", profile.custom_prompt, prompt)
            } else {
                prompt
            };

            let config = PoolConfig {
                max_concurrent,
                total,
                progress_event: "video-progress",
                log_tag: "video_queue",
            };

            let app_pool = app.clone();
            let token_pool = token.clone();
            let pause_pool = pause.clone();

            let process_one = move |job: VideoJob| {
                let client = client.clone();
                let settings = settings.clone();
                let app = app.clone();
                let token = token.clone();
                let video_prompt = video_prompt.clone();
                let video_settings = video_settings.clone();
                async move {
                    process_video_job(job, &client, &settings, &app, &token, &video_prompt, &video_settings).await
                }
            };

            pool::run_worker_pool(app_pool, jobs, token_pool, Some(pause_pool), config, process_one).await;
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
        // Kein cleanup_temp_dirs() hier — das würde alle vergangenen
        // Reasoning-Dateien löschen. Cleanup passiert nur beim App-Start.
    }
}

/// Kleiner Helfer: baut ein VideoJobUpdateEvent mit den gemeinsamen Defaults.
fn status_event(job_id: &str, status: VideoJobStatus, error_msg: Option<String>) -> VideoJobUpdateEvent {
    VideoJobUpdateEvent {
        job_id: job_id.to_string(),
        status,
        tags: vec![],
        description: String::new(),
        genres: vec![],
        title: String::new(),
        error_msg,
    }
}

/// Verarbeitet einen einzelnen Video-Job: Frames extrahieren → AI-Analyse → JSON parsen →
/// (optional) Metadaten schreiben → Events. Der Rückgabewert steuert die Fortschrittszählung.
async fn process_video_job(
    mut job: VideoJob,
    client: &reqwest::Client,
    settings: &AppSettings,
    app: &AppHandle,
    token: &CancellationToken,
    video_prompt: &str,
    video_settings: &VideoSettings,
) -> (VideoJob, JobOutcome) {
    let temp_dir = std::env::temp_dir().join("meta-analyzer").join(&job.id);

    if let Err(e) = std::fs::create_dir_all(&temp_dir) {
        let _ = app.emit("video-job-update", status_event(&job.id, VideoJobStatus::Error, Some(format!("Failed to create temp dir: {}", e))));
        return (job, JobOutcome::DoneNoProgress);
    }

    if token.is_cancelled() {
        return (job, JobOutcome::Cancelled);
    }

    // ── Phase 1: Frames extrahieren (oder aus vorherigem Durchlauf wiederverwenden) ──
    let frames_ready = !job.frames.is_empty() && job.frames.len() == video_settings.num_frames as usize;
    if frames_ready {
        eprintln!("[video_queue] Reusing {} existing frames for {}", job.frames.len(), job.file_name);
    } else {
        job.status = VideoJobStatus::Extracting;
        let _ = app.emit("video-job-update", status_event(&job.id, VideoJobStatus::Extracting, None));

        match video_decoder::extract_keyframes(
            &job.path,
            &temp_dir,
            video_settings.num_frames,
            video_settings.frame_width,
            video_settings.frame_height,
        ) {
            Ok(frames) => {
                job.frames = frames;
                let total_frames = job.frames.len();
                eprintln!("[video_queue] Extracted {} frames for {}", total_frames, job.file_name);
                let _ = app.emit("video-frame-extracted", VideoFrameExtractedEvent {
                    job_id: job.id.clone(),
                    total: total_frames,
                    completed: total_frames,
                    frames: job.frames.clone(),
                });
            }
            Err(e) => {
                let _ = app.emit("video-job-update", status_event(&job.id, VideoJobStatus::Error, Some(format!("Frame extraction failed: {}", e))));
                let _ = std::fs::remove_dir_all(&temp_dir);
                return (job, JobOutcome::DoneNoProgress);
            }
        }
    }

    // ── Phase 2: AI-Analyse (Haupt-Prompt) ──
    job.status = VideoJobStatus::Processing;
    let _ = app.emit("video-job-update", status_event(&job.id, VideoJobStatus::Processing, None));

    let frame_paths: Vec<String> = job.frames.iter().map(|f| f.path.clone()).collect();

    // Dateinamen als Referenz für Titel und Kontext an den Prompt anhängen.
    let prompt_with_filename = if video_settings.write_title {
        format!(
            "{}\n\nFILE REFERENCE:\nThe raw filename is: {}\n\
            CRITICAL: Extract any usable information from this filename and incorporate it into the title:\n\
            - Names of people, characters, or performers visible in the filename\n\
            - Series, franchise, or channel names\n\
            - Scene themes, activities, or categories\n\
            - Episode numbers, years, locations, or other identifiers\n\
            - Ignore file extensions (.mp4, .mkv, etc.), resolution tags (1080p, 4K), and quality markers (HD, WEB-DL, etc.)\n\
            The title MUST reflect the actual content visible in both the filename (if matches) and the video frames.\n\
            Be creative and unique — avoid generic or template-like titles. Use cluses from texts visible in the frames, but also leverage any hints from the filename.",
            video_prompt,
            job.file_name,
        )
    } else {
        video_prompt.to_string()
    };

    let result = ai_client::analyze_video_frames(
        client, settings, &frame_paths, app, &job.id, &prompt_with_filename, &temp_dir, Some(token.clone()),
    ).await;

    // Bei Fehler: 1× Retry (nur wenn nicht durch Cancel verursacht!)
    let result = match result {
        Ok(r) => Ok(r),
        Err(first_err) => {
            if token.is_cancelled() {
                Err(first_err)
            } else {
                eprintln!("[video_queue] First attempt failed ({}), retrying…", first_err);
                ai_client::analyze_video_frames(
                    client, settings, &frame_paths, app, &job.id, &prompt_with_filename, &temp_dir, Some(token.clone()),
                ).await
            }
        }
    };

    let (status, tags, error_msg, description, genres, title) = match result {
        Ok(response) => {
            let response = response.trim();
            // Roh-String bereinigen: Label-Prefixe + GLM/Qwen-Control-Marker entfernen.
            let cleaned = strip_response_label(response)
                .replace("<|begin_of_box|>", "")
                .replace("<|end_of_box|>", "")
                .trim()
                .to_string();

            match serde_json::from_str::<VideoMetaOutput>(&cleaned) {
                Ok(meta) => {
                    let meta_title = if video_settings.write_title && !meta.title.is_empty() {
                        meta.title.clone()
                    } else {
                        String::new()
                    };

                    if video_settings.apply_automatically {
                        let meta_with_title = VideoMetaOutput { title: meta_title.clone(), ..meta.clone() };
                        match video_metadata::write_video_metadata(
                            &job.path,
                            &meta_with_title,
                            video_settings.write_description,
                            video_settings.write_genres,
                            video_settings.write_tags,
                            video_settings.write_title,
                        ) {
                            Err(e) => {
                                eprintln!("[video_queue] Metadata write failed: {}", e);
                                (VideoJobStatus::Error, meta.keywords, Some(format!("Metadata write failed: {}", e)), meta.description.clone(), meta.genres.clone(), meta_title)
                            }
                            Ok(_) => (VideoJobStatus::Done, meta.keywords, None, meta.description.clone(), meta.genres.clone(), meta_title),
                        }
                    } else {
                        (VideoJobStatus::DonePending, meta.keywords, None, meta.description.clone(), meta.genres.clone(), meta_title)
                    }
                }
                Err(_) => {
                    // Fallback: alter comma-separated Weg.
                    eprintln!("[video_queue] LLM returned invalid JSON, falling back to comma parse");
                    let tags: Vec<String> = response.split(',').map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).collect();
                    if video_settings.apply_automatically {
                        match video_metadata::write_video_metadata_fallback(
                            &job.path,
                            &tags,
                            video_settings.write_description,
                            video_settings.write_genres,
                            video_settings.write_tags,
                            video_settings.write_title,
                        ) {
                            Err(e) => (VideoJobStatus::Error, tags, Some(format!("Metadata write failed: {}", e)), String::new(), vec![], String::new()),
                            Ok(_) => (VideoJobStatus::Done, tags, None, String::new(), vec![], String::new()),
                        }
                    } else {
                        (VideoJobStatus::DonePending, tags, None, String::new(), vec![], String::new())
                    }
                }
            }
        }
        Err(_e) if token.is_cancelled() => {
            eprintln!("[video_queue] Cancelled job {} — resetting to pending", job.file_name);
            let _ = app.emit("video-job-update", status_event(&job.id, VideoJobStatus::Pending, None));
            return (job, JobOutcome::Cancelled);
        }
        Err(e) => (VideoJobStatus::Error, vec![], Some(e.to_string()), String::new(), vec![], String::new()),
    };

    // Strukturierte Ergebnisse in den Job schreiben, damit sie ein Apply-Round-Trip überleben.
    job.tags = tags;
    job.description = description.clone();
    job.genres = genres.clone();
    job.title = title.clone();

    let _ = app.emit("video-job-update", VideoJobUpdateEvent {
        job_id: job.id.clone(),
        status,
        tags: job.tags.clone(),
        description,
        genres,
        title,
        error_msg,
    });

    (job, JobOutcome::Done)
}

/// Entfernt Label-Prefixe wie "Answer:", "Keywords:", "Tags:" aus dem Roh-Output.
fn strip_response_label(s: &str) -> String {
    let s = s.trim();
    let lower = s.to_lowercase();
    for prefix in &["answer:", "keywords:", "tags:", "keyword:", "tag:", "result:"] {
        if let Some(rest) = lower.strip_prefix(prefix) {
            return rest.trim().to_string();
        }
    }
    s.to_string()
}
