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

//! Generischer Worker-Pool für die Foto- und Video-Verarbeitung.
//!
//! Die *Orchestrierung* — N Worker, Semaphore, optionale Pause, kooperatives Cancel,
//! Job-Auswahl aus der Pending-Liste, Zurücksetzen abgebrochener Jobs und die
//! Fortschrittszählung — lebt hier an EINER Stelle. Die job-spezifische Arbeit
//! (Analyse, Metadaten schreiben, Events emittieren) kommt als Closure `process_one` rein.
//!
//! Cancellation ist kooperativ: Die AI-Aufrufe bekommen den `CancellationToken` und geben
//! bei STOP einen Fehler zurück; die Tasks werden nie hart abgebrochen. Deshalb genügt es,
//! den Job per Rückgabewert zurückzusetzen — ein Drop-Guard ist nicht nötig.

use std::future::Future;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{Mutex, Notify, Semaphore};
use tokio_util::sync::CancellationToken;
use tauri::{AppHandle, Emitter};
use crate::models::{ProgressEvent, ProgressTimerEvent};

/// Ein Job, den der generische Pool verwalten kann.
pub trait PoolJob: Clone + Send + 'static {
    fn is_pending(&self) -> bool;
    /// Setzt den Job zurück auf Pending (wird bei Cancel aufgerufen).
    fn reset_to_pending(&mut self);
}

/// Pause-Steuerung (nur der Foto-Pool nutzt sie; Video übergibt `None`).
#[derive(Clone)]
pub struct PauseControl {
    pub is_paused: Arc<Mutex<bool>>,
    pub notify: Arc<Notify>,
}

/// Ergebnis der job-spezifischen Verarbeitung.
pub enum JobOutcome {
    /// Job verarbeitet (Done/Error/DonePending) → Fortschritt zählen, nächster Job.
    Done,
    /// Job beendet, aber NICHT für den Fortschritt zählen (z.B. früher Fehler vor der
    /// eigentlichen Analyse — erhält das bestehende Verhalten der Video-Pipeline).
    DoneNoProgress,
    /// Durch STOP abgebrochen → Job zurücksetzen, Worker beenden.
    Cancelled,
}

pub struct PoolConfig {
    pub max_concurrent: usize,
    pub total: usize,
    /// Event-Name für den Fortschritt ("progress" bzw. "video-progress").
    /// Beide Payloads sind identisch (`{completed, total}`), nur der Name unterscheidet sich.
    pub progress_event: &'static str,
    /// Präfix für Log-Ausgaben, z.B. "queue" / "video_queue".
    pub log_tag: &'static str,
}

/// Startet die Worker, verteilt die Jobs und wartet, bis alle fertig sind.
///
/// `process_one` erhält einen Job und liefert `(Job, JobOutcome)` zurück. Es ist selbst
/// dafür verantwortlich, seine Status-/Ergebnis-Events zu emittieren; der Pool kümmert sich
/// nur um Orchestrierung und den gemeinsamen Fortschritts-/Timer-Event.
pub async fn run_worker_pool<J, F, Fut>(
    app: AppHandle,
    jobs: Vec<J>,
    token: CancellationToken,
    pause: Option<PauseControl>,
    config: PoolConfig,
    process_one: F,
) where
    J: PoolJob,
    F: Fn(J) -> Fut + Clone + Send + 'static,
    Fut: Future<Output = (J, JobOutcome)> + Send + 'static,
{
    let start_instant = Arc::new(Instant::now());
    let pending: Arc<Mutex<Vec<J>>> = Arc::new(Mutex::new(jobs));
    let completed = Arc::new(Mutex::new(0usize));
    let semaphore = Arc::new(Semaphore::new(config.max_concurrent));
    let config = Arc::new(config);

    let mut handles = Vec::new();

    for _worker_id in 0..config.max_concurrent {
        let token = token.clone();
        let pause = pause.clone();
        let app = app.clone();
        let pending = pending.clone();
        let completed = completed.clone();
        let semaphore = semaphore.clone();
        let start_instant = start_instant.clone();
        let config = config.clone();
        let process_one = process_one.clone();

        let handle = tokio::spawn(async move {
            loop {
                if token.is_cancelled() { break; }

                if let Some(ref p) = pause {
                    wait_if_paused(&token, p).await;
                    if token.is_cancelled() { break; }
                }

                let permit = semaphore.acquire().await;
                if token.is_cancelled() { drop(permit); break; }

                // Nächsten Pending-Job aus der gemeinsamen Liste ziehen.
                let job = {
                    let mut list = pending.lock().await;
                    match list.iter().position(|j| j.is_pending()) {
                        Some(idx) => Some(list.remove(idx)),
                        None => None,
                    }
                };
                let job = match job {
                    Some(j) => j,
                    None => { drop(permit); break; } // nichts mehr zu tun
                };

                // Job-spezifische Verarbeitung.
                let (mut job, outcome) = process_one(job).await;

                match outcome {
                    JobOutcome::Done => {
                        let mut c = completed.lock().await;
                        *c += 1;
                        let elapsed = start_instant.elapsed().as_secs_f64();
                        let avg = if *c > 0 { elapsed / *c as f64 } else { 0.0 };
                        let _ = app.emit(config.progress_event, ProgressEvent {
                            completed: *c,
                            total: config.total,
                        });
                        let _ = app.emit("progress-timer", ProgressTimerEvent {
                            start_time: elapsed,
                            avg_seconds_per_job: avg,
                        });
                    }
                    JobOutcome::DoneNoProgress => {}
                    JobOutcome::Cancelled => {
                        job.reset_to_pending();
                        pending.lock().await.push(job);
                        drop(permit);
                        break;
                    }
                }

                drop(permit);
            }
        });
        handles.push(handle);
    }

    for h in handles { let _ = h.await; }
    eprintln!("[{}] All workers done, total={}", config.log_tag, config.total);
}

/// Blockiert, solange pausiert ist — reagiert dabei sofort auf Cancel.
async fn wait_if_paused(token: &CancellationToken, pause: &PauseControl) {
    loop {
        if token.is_cancelled() { break; }
        if !*pause.is_paused.lock().await { break; }
        tokio::select! {
            _ = pause.notify.notified() => {}
            _ = token.cancelled() => { break; }
        }
    }
}
