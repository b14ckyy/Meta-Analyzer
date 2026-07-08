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

use anyhow::{anyhow, Result};
use base64::prelude::*;
use futures_util::StreamExt;
use reqwest::Client;
use serde_json::json;
use std::io::Cursor;
use tauri::{AppHandle, Emitter};
use tokio_util::sync::CancellationToken;
use crate::models::{AppSettings, StreamChunkEvent};

/// Maximum pixel dimension (width or height) for images sent to the API.
/// Large images cause OOM or timeout errors in the vision model.
const MAX_DIMENSION: u32 = 1500;

/// Ergebnis eines gestreamten Chat-Completion-Calls.
struct StreamResult {
    content: String,
    reasoning: String,
    /// true, wenn der Stream durch den CancellationToken (STOP) abgebrochen wurde.
    cancelled: bool,
    /// (prompt, completion, total) — nur gesetzt, wenn das Backend usage liefert.
    usage: Option<(u64, u64, u64)>,
}

impl StreamResult {
    /// Der zu parsende Text: bevorzugt `content`, sonst `reasoning`
    /// (manche Reasoning-Modelle liefern die finale Antwort nur im reasoning_content).
    fn parse_source(&self) -> &str {
        if self.content.trim().is_empty() {
            self.reasoning.trim()
        } else {
            self.content.trim()
        }
    }
}

/// Gemeinsamer Streaming-Kern für Bild- und Video-Analyse.
///
/// Sendet ein bereits fertig gebautes Chat-Completion-`payload` (stream=true) an das
/// Endpoint, parst den SSE-Stream inkrementell und emittiert `stream-chunk`-Events für
/// reasoning/content. Reagiert per `select!` sofort auf Cancellation. Das feld-spezifische
/// Bauen des Payloads und das Weiterverarbeiten des Ergebnisses bleibt beim Aufrufer.
async fn stream_chat_completion(
    client: &Client,
    settings: &AppSettings,
    payload: serde_json::Value,
    app: &AppHandle,
    job_id: &str,
    cancel_token: Option<CancellationToken>,
) -> Result<StreamResult> {
    let endpoint = format!(
        "{}/v1/chat/completions",
        settings.api_url.trim_end_matches('/')
    );

    let mut request = client.post(&endpoint).json(&payload);
    if !settings.api_key.trim().is_empty() {
        request = request.bearer_auth(settings.api_key.trim());
    }
    let response = request.send().await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow!("LM Studio API error {}: {}", status, body));
    }

    use tokio::select;
    let mut stream = response.bytes_stream();
    let mut content = String::new();
    let mut reasoning = String::new();
    let mut buffer = String::new();
    let mut pending_bytes: Vec<u8> = Vec::new();
    let mut usage: Option<(u64, u64, u64)> = None;
    let mut cancelled = false;

    eprintln!("[ai_client] Stream opened for job {}", job_id);

    loop {
        let next_chunk = select! {
            biased;

            _ = async {
                if let Some(ref ct) = cancel_token {
                    ct.cancelled().await
                } else {
                    std::future::pending::<()>().await
                }
            } => {
                eprintln!("[ai_client] Cancelled stream for job {}", job_id);
                cancelled = true;
                break;
            }

            chunk = stream.next() => chunk,
        };

        let chunk = match next_chunk {
            Some(Ok(c)) => c,
            Some(Err(e)) => return Err(anyhow!("Stream error: {}", e)),
            None => break, // Stream ended normally
        };

        // UTF-8-Grenzen über Chunk-Grenzen hinweg korrekt behandeln.
        pending_bytes.extend_from_slice(&chunk);
        let (valid_len, rest_bytes) = match std::str::from_utf8(&pending_bytes) {
            Ok(_) => (pending_bytes.len(), Vec::new()),
            Err(e) => {
                let v = e.valid_up_to();
                (v, pending_bytes[v..].to_vec())
            }
        };
        let decoded = std::str::from_utf8(&pending_bytes[..valid_len])
            .unwrap_or("")
            .to_string();
        pending_bytes = rest_bytes;
        buffer.push_str(&decoded);

        // Vollständige SSE-Event-Blöcke (getrennt durch Leerzeile) abarbeiten.
        loop {
            let end = match buffer.find("\n\n").or_else(|| buffer.find("\r\n\r\n")) {
                Some(i) => i,
                None => break,
            };
            let event_block: String = buffer.drain(..end).collect();
            if buffer.starts_with("\r\n\r\n") {
                buffer.drain(..4);
            } else {
                buffer.drain(..2);
            }

            for line in event_block.lines() {
                let data = match line.strip_prefix("data: ").or_else(|| line.strip_prefix("data:")) {
                    Some(d) => d.trim(),
                    None => continue,
                };
                if data == "[DONE]" || data.is_empty() {
                    continue;
                }
                let json: serde_json::Value = match serde_json::from_str(data) {
                    Ok(j) => j,
                    Err(_) => continue,
                };

                // Usage-Info (letzter Chunk vor [DONE]) mitnehmen, falls vorhanden.
                if let Some(u) = json.get("usage") {
                    if let (Some(p), Some(c), Some(t)) = (
                        u.get("prompt_tokens").and_then(|v| v.as_u64()),
                        u.get("completion_tokens").and_then(|v| v.as_u64()),
                        u.get("total_tokens").and_then(|v| v.as_u64()),
                    ) {
                        usage = Some((p, c, t));
                    }
                }

                let delta = &json["choices"][0]["delta"];

                if let Some(r) = delta["reasoning_content"].as_str() {
                    if !r.is_empty() {
                        reasoning.push_str(r);
                        let _ = app.emit("stream-chunk", StreamChunkEvent {
                            job_id: job_id.to_string(),
                            kind: "reasoning".to_string(),
                            delta: r.to_string(),
                        });
                    }
                }

                if let Some(c) = delta["content"].as_str() {
                    if !c.is_empty() {
                        content.push_str(c);
                        let _ = app.emit("stream-chunk", StreamChunkEvent {
                            job_id: job_id.to_string(),
                            kind: "content".to_string(),
                            delta: c.to_string(),
                        });
                    }
                }
            }
        }
    }

    Ok(StreamResult { content, reasoning, cancelled, usage })
}

pub async fn analyze_image(
    client: &Client,
    settings: &AppSettings,
    image_path: &str,
    app: &AppHandle,
    job_id: &str,
    prompt: &str,
    cancel_token: Option<CancellationToken>,
) -> Result<Vec<String>> {
    // Read image, optionally resize to MAX_DIMENSION, and encode as base64 data URL
    let data_url = load_and_resize_image(image_path).await?;

    let payload = json!({
        "model": settings.model_name,
        "stream": true,
        "messages": [
            {
                "role": "user",
                "content": [
                    { "type": "image_url", "image_url": { "url": data_url, "detail": "low" } },
                    { "type": "text", "text": prompt }
                ]
            }
        ]
    });

    eprintln!("[ai_client] PROMPT for {}:\n---\n{}\n---", job_id, prompt);

    let result = stream_chat_completion(client, settings, payload, app, job_id, cancel_token).await?;

    // Wenn durch STOP abgebrochen → sofort Fehler zurück, kein Parsing des Teil-Results!
    if result.cancelled {
        return Err(anyhow!("Request cancelled by user (STOP pressed)"));
    }

    // Usage-Event emittieren (nur Foto-Pfad nutzt Tokens/s im UI).
    if let Some((prompt_tokens, completion, total)) = result.usage {
        if total > 0 {
            let usage_json = serde_json::json!({
                "prompt": prompt_tokens, "completion": completion, "total": total,
            });
            let _ = app.emit("stream-chunk", StreamChunkEvent {
                job_id: job_id.to_string(),
                kind: "usage".to_string(),
                delta: usage_json.to_string(),
            });
        }
    }

    let tags = parse_tags(result.parse_source());
    if tags.is_empty() {
        let raw_display = if result.content.trim().is_empty() {
            format!("[reasoning_content]\n{}", result.reasoning.trim())
        } else {
            format!("[content]\n{}", result.content.trim())
        };
        return Err(anyhow!("No tags parsed from response.\n{}", raw_display));
    }
    Ok(tags)
}

/// Read an image from disk, resize it so that the longest side is at most
/// MAX_DIMENSION (preserving aspect ratio), and return a base64 data URL.
async fn load_and_resize_image(image_path: &str) -> Result<String> {
    let image_bytes = tokio::fs::read(image_path).await?;

    // Determine MIME type from file extension
    let ext = std::path::Path::new(image_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("jpeg")
        .to_lowercase();
    let is_png = ext == "png";
    let mime = if is_png { "image/png" } else { "image/jpeg" };

    // Decode image to get dimensions and optionally resize
    let img = match image::load_from_memory(&image_bytes) {
        Ok(i) => i,
        Err(_) => {
            // If we can't decode as image, fall back to raw bytes (e.g. for WebP etc.)
            // This is a safety net — the API might still accept it.
            let b64 = BASE64_STANDARD.encode(&image_bytes);
            return Ok(format!("data:{mime};base64,{b64}"));
        }
    };

    let (w, h) = (img.width(), img.height());

    // Only resize if any dimension exceeds MAX_DIMENSION
    let resized = if w > MAX_DIMENSION || h > MAX_DIMENSION {
        let new_w: u32;
        let new_h: u32;
        if w > h {
            new_w = MAX_DIMENSION;
            new_h = (h as f64 * MAX_DIMENSION as f64 / w as f64).round() as u32;
        } else {
            new_h = MAX_DIMENSION;
            new_w = (w as f64 * MAX_DIMENSION as f64 / h as f64).round() as u32;
        }
        eprintln!(
            "[ai_client] Resizing {} from {}x{} to {}x{}",
            image_path, w, h, new_w, new_h
        );
        // Lanczos3 filter gives best quality for downscaling
        Some(img.resize_exact(new_w.max(1), new_h.max(1), image::imageops::FilterType::Lanczos3))
    } else {
        None
    };

    let final_img = resized.as_ref().unwrap_or(&img);

    // Encode back to bytes
    let mut encoded_bytes: Vec<u8> = Vec::new();
    if is_png {
        final_img.write_to(&mut Cursor::new(&mut encoded_bytes), image::ImageFormat::Png)?;
    } else {
        // JPEG quality 85 — good balance of size and quality for vision models
        let mut jpeg_encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut encoded_bytes, 85);
        jpeg_encoder.encode(
            final_img.as_bytes(),
            final_img.width(),
            final_img.height(),
            final_img.color().into(),
        )?;
    }

    let b64 = BASE64_STANDARD.encode(&encoded_bytes);
    Ok(format!("data:{mime};base64,{b64}"))
}

/// Extract tags from raw model output. Handles many real-world output formats:
/// comma-separated, newline-separated, numbered lists, bracketed output,
/// labeled output ("Keywords: ..."), quoted tags, and GLM/Qwen control markers.
fn parse_tags(raw: &str) -> Vec<String> {
    // Strip known control markers (GLM, Qwen, etc.)
    let cleaned = raw
        .replace("<|begin_of_box|>", "")
        .replace("<|end_of_box|>", "")
        .replace("<|box_start|>", "")
        .replace("<|box_end|>", "")
        .replace("<think>", "")
        .replace("</think>", "");

    let s = cleaned.trim();

    // Strip leading label: "Keywords:", "Tags:", "Answer:", etc.
    let s = {
        let lower = s.to_lowercase();
        let mut result = s;
        for prefix in &["keywords:", "tags:", "keyword:", "tag:", "answer:", "result:"] {
            if lower.starts_with(prefix) {
                result = s[prefix.len()..].trim();
                break;
            }
        }
        result
    };

    // Strip surrounding brackets [ ] or ( )
    let s = if s.len() >= 2
        && ((s.starts_with('[') && s.ends_with(']'))
            || (s.starts_with('(') && s.ends_with(')')))
    {
        &s[1..s.len() - 1]
    } else {
        s
    };

    // Use comma as separator if present, otherwise fall back to newlines
    let parts: Vec<&str> = if s.contains(',') {
        s.split(',').collect()
    } else {
        s.lines().collect()
    };

    parts
        .iter()
        .map(|t| {
            let t = t.trim();
            // Strip leading numbering/bullets: "1.", "1)", "-", "*"
            let t = t
                .trim_start_matches(|c: char| c.is_ascii_digit())
                .trim_start_matches(|c: char| matches!(c, '.' | ')' | '-' | '*'))
                .trim();
            // Strip surrounding quotes
            t.trim_matches(|c: char| matches!(c, '"' | '\'' | '`')).trim()
        })
        .filter(|t| {
            if t.is_empty() { return false; }
            if t.contains("<|") || t.contains("|>") { return false; }
            // Reject anything that looks like a sentence rather than a keyword:
            // – too long (real tags are short phrases)
            if t.len() > 50 { return false; }
            // – ends with sentence-terminating punctuation
            if t.ends_with('.') || t.ends_with('?') || t.ends_with('!') { return false; }
            // – contains mid-string sentence punctuation
            if t.contains(". ") || t.contains("? ") || t.contains("! ") { return false; }
            true
        })
                .map(|t| t.to_lowercase())
        .collect()
}

/// Analysiere mehrere Video-Frames in einem API-Call.
/// Sendet alle Frames als multi-image content block an das LLM.
/// Gibt den ROH-Content zurück (String) — das Parsing übernimmt der Aufrufer.
/// Speichert zusätzlich das Reasoning in {reasoning_dir}/{job_id}.reasoning.txt.
pub async fn analyze_video_frames(
    client: &Client,
    settings: &AppSettings,
    frame_paths: &[String],
    app: &AppHandle,
    job_id: &str,
    prompt: &str,
    reasoning_dir: &std::path::Path,
    cancel_token: Option<CancellationToken>,
) -> Result<String> {
    // Alle Frames als image_url content blocks + text prompt
    let mut content: Vec<serde_json::Value> = Vec::with_capacity(frame_paths.len() + 1);
    for path in frame_paths {
        let data_url = load_and_resize_image(path).await?;
        content.push(json!({
            "type": "image_url",
            "image_url": { "url": data_url, "detail": "low" }
        }));
    }
    content.push(json!({ "type": "text", "text": prompt }));

    let payload = json!({
        "model": settings.model_name,
        "stream": true,
        "messages": [ { "role": "user", "content": content } ]
    });

    eprintln!(
        "[ai_client] analyze_video_frames: {} frames, prompt length={}",
        frame_paths.len(),
        prompt.len()
    );

    let result = stream_chat_completion(client, settings, payload, app, job_id, cancel_token).await?;

    // Wenn durch STOP abgebrochen → sofort Fehler zurück, kein Speichern/Parsen
    if result.cancelled {
        return Err(anyhow!("Request cancelled by user (STOP pressed)"));
    }

    let parse_source = result.parse_source().to_string();

    // Reasoning + Content + finalen Output zusammen speichern (für "Show Reasoning" in der UI).
    let log_content = format!(
        "[REASONING]\n{}\n\n[CONTENT]\n{}\n\n[OUTPUT]\n{}",
        result.reasoning.trim(),
        result.content.trim(),
        parse_source
    );
    if let Err(e) = std::fs::write(
        reasoning_dir.join(format!("{}.reasoning.txt", job_id)),
        &log_content,
    ) {
        eprintln!("[ai_client] Failed to save reasoning log for {}: {}", job_id, e);
    }

    if parse_source.is_empty() {
        return Err(anyhow!("Empty response from video analysis.\n[content]\n{}\n[reasoning]\n{}",
            result.content.trim(), result.reasoning.trim()));
    }

    Ok(parse_source)
}
