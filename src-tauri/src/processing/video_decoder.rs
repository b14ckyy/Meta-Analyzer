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
use std::path::{Path, PathBuf};
use std::process::Command;
use crate::video_models::VideoFrame;

// ── Binary Discovery ──────────────────────────────────────────────
//
// Such-Reihenfolge:
//   1. Neben der Meta-Analyzer-Exe (d.h. im selben Verzeichnis)
//   2. Im PATH-System
//
// Dadurch kann man ffmpeg/ffprobe einfach neben die App legen,
// was das Bundling und portable Nutzung ermöglicht.

static FFMPEG_NAME: &str = "ffmpeg";
static FFPROBE_NAME: &str = "ffprobe";

/// Finde den ffmpeg-Pfad (zuerst App-Verzeichnis, dann PATH).
fn find_ffmpeg() -> Option<PathBuf> {
    find_binary(FFMPEG_NAME)
}

/// Finde den ffprobe-Pfad (zuerst App-Verzeichnis, dann PATH).
fn find_ffprobe() -> Option<PathBuf> {
    find_binary(FFPROBE_NAME)
}

/// Generische Suche: schaue zuerst neben der eigenen Exe, dann im PATH.
fn find_binary(name: &str) -> Option<PathBuf> {
    // 1. Neben der eigenen Exe
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let side_path = exe_dir.join(if cfg!(windows) {
                format!("{}.exe", name)
            } else {
                name.to_string()
            });
            if side_path.exists() {
                return Some(side_path);
            }

            // Windows: manche User haben es ohne .exe im Ordner (selten)
            #[cfg(windows)]
            {
                let side_path_no_ext = exe_dir.join(name);
                if side_path_no_ext.exists() {
                    return Some(side_path_no_ext);
                }
            }
        }
    }

    // 2. PATH-Suche
    if let Ok(paths) = std::env::var("PATH") {
        for dir in std::env::split_paths(&paths) {
            let candidate = dir.join(if cfg!(windows) {
                format!("{}.exe", name)
            } else {
                name.to_string()
            });
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }

    None
}

/// Prevent Windows from flashing a console window for every ffmpeg/ffprobe child
/// process. Without this, each spawn from the GUI/release build (windows
/// subsystem) briefly pops a console window. No-op on non-Windows platforms.
pub(crate) fn no_window(cmd: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    #[cfg(not(windows))]
    {
        let _ = cmd;
    }
}

/// Erstelle einen Befehl mit dem gefundenen ffmpeg-Binary.
fn ffmpeg_cmd() -> Result<Command> {
    let path = find_ffmpeg()
        .ok_or_else(|| anyhow!("ffmpeg not found — neither next to the app nor in PATH"))?;
    let mut cmd = Command::new(path);
    no_window(&mut cmd);
    Ok(cmd)
}

/// Erstelle einen Befehl mit dem gefundenen ffprobe-Binary.
fn ffprobe_cmd() -> Result<Command> {
    let path = find_ffprobe()
        .ok_or_else(|| anyhow!("ffprobe not found — neither next to the app nor in PATH"))?;
    let mut cmd = Command::new(path);
    no_window(&mut cmd);
    Ok(cmd)
}

// ── Public API ────────────────────────────────────────────────────

/// Extrahiere N Frames aus einem Video via ffmpeg Keyframe-Seek.
///
/// Strategie:
/// 1. Dauer via ffprobe ermitteln
/// 2. N gleichmäßig verteilte Timestamps berechnen
/// 3. Pro Timestamp: ffmpeg -ss POS -i video -frames:v 1
///    → ffmpeg sucht automatisch den nächsten Keyframe VOR der Position
///    → Extrem schnell (10-50ms pro Frame, kein full decode)
///
/// Alle Frames werden auf max_dimension × max_dimension skaliert (Seitenverhältnis erhalten).
pub fn extract_keyframes(
    video_path: &str,
    output_dir: &Path,
    num_frames: u8,
    max_width: u32,
    max_height: u32,
) -> Result<Vec<VideoFrame>> {
    let duration = get_duration(video_path)?;
    let num = (num_frames as usize).max(1).min(50);
    let mut frames = Vec::with_capacity(num);

    eprintln!(
        "[video_decoder] Extracting {} frames from {} (duration={}s)",
        num, video_path, duration
    );

    for i in 0..num {
        // Berechne Ziel-Timestamp (gleichmäßig verteilt über die Videodauer)
        let target_secs = if num == 1 {
            duration / 2.0 // Nur 1 Frame → Mitte vom Video
        } else {
            (duration / (num as f64)) * (i as f64)
        };

        let timestamp_str = format_timestamp(target_secs);
        let output_path = output_dir.join(format!("frame_{:03}.jpg", i));

        // ffmpeg Keyframe-Seek: -ss VOR -i → springt zum nächsten Keyframe
        let scale_filter = format!(
            "scale='min({},iw)':min'({},ih)':force_original_aspect_ratio=decrease",
            max_width, max_height
        );

        let mut cmd = ffmpeg_cmd()?;
        let status = cmd
            .args([
                "-ss", &timestamp_str,
                "-i", video_path,
                "-frames:v", "1",
                "-vf", &scale_filter,
                "-q:v", "2", // JPEG-Qualität (2 = very high, scale 2-31)
                "-y",        // Überschreiben ohne Nachfrage
            ])
            .arg(&output_path)
            .output()
            .map_err(|e| anyhow!("ffmpeg execution failed: {}", e))?;

        if !status.status.success() {
            let stderr = String::from_utf8_lossy(&status.stderr);
            // Wenn ffmpeg fehlschlägt (z.B. kein Video-Stream), Fehler geben
            return Err(anyhow!(
                "ffmpeg failed at timestamp {}: {}",
                timestamp_str,
                stderr.lines().last().unwrap_or("unknown error")
            ));
        }

        frames.push(VideoFrame {
            index: i as u32,
            path: output_path.to_string_lossy().to_string(),
            timestamp_secs: target_secs,
        });

        eprintln!(
            "[video_decoder]  Frame {:02}/{:02} at {}s ({})",
            i + 1,
            num,
            target_secs,
            output_path.display()
        );
    }

    Ok(frames)
}

/// Versuche, ein eingebettetes Cover-Bild (`attached_pic`, z.B. bei MP4/MKV mit
/// Poster) zu extrahieren. Gibt Ok(true) zurück, wenn ein Cover geschrieben wurde.
pub fn extract_embedded_thumbnail(video_path: &str, output_path: &Path) -> Result<bool> {
    // 1) Per ffprobe prüfen, ob es einen attached_pic-Videostream gibt.
    let mut probe = ffprobe_cmd()?;
    let out = probe
        .args([
            "-v", "error",
            "-select_streams", "v",
            "-show_entries", "stream=index:stream_disposition=attached_pic",
            "-of", "json",
            video_path,
        ])
        .output()
        .map_err(|e| anyhow!("ffprobe execution failed: {}", e))?;

    if !out.status.success() {
        return Ok(false);
    }

    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap_or_default();
    let mut pic_index: Option<i64> = None;
    if let Some(streams) = json["streams"].as_array() {
        for s in streams {
            if s["disposition"]["attached_pic"].as_i64() == Some(1) {
                pic_index = s["index"].as_i64();
                break;
            }
        }
    }

    let idx = match pic_index {
        Some(i) => i,
        None => return Ok(false),
    };

    // 2) Den Cover-Stream als Bild rausschreiben.
    let mut cmd = ffmpeg_cmd()?;
    let status = cmd
        .args(["-i", video_path, "-map", &format!("0:{}", idx), "-frames:v", "1", "-y"])
        .arg(output_path)
        .output()
        .map_err(|e| anyhow!("ffmpeg cover extraction failed: {}", e))?;

    Ok(status.status.success() && output_path.exists())
}

/// Extrahiere `count` gleichmäßig über die Videodauer verteilte Vorschau-Frames.
/// Die Zeitpunkte sind zentriert ((i+0.5)/count), um Schwarzbilder am Anfang/Ende
/// zu vermeiden. Für die Listen-Vorschau, bevor die volle Frame-Extraktion läuft.
pub fn extract_thumbnails(
    video_path: &str,
    out_dir: &Path,
    count: u8,
    max_w: u32,
    max_h: u32,
) -> Result<Vec<PathBuf>> {
    let duration = get_duration(video_path).unwrap_or(0.0);
    let n = count.max(1) as usize;
    // Sauberer Downscale, passt ins max_w×max_h-Rechteck (kein Upscale-Zwang, keine Quoting-Fallen).
    let scale_filter = format!(
        "scale=w={}:h={}:force_original_aspect_ratio=decrease",
        max_w, max_h
    );

    let mut paths = Vec::with_capacity(n);
    for i in 0..n {
        let target_secs = if duration > 0.5 {
            duration * ((i as f64 + 0.5) / n as f64)
        } else {
            0.0
        };
        let out = out_dir.join(format!("thumb_{:02}.jpg", i));

        let mut cmd = ffmpeg_cmd()?;
        let status = cmd
            .args([
                "-ss", &format_timestamp(target_secs),
                "-i", video_path,
                "-frames:v", "1",
                "-vf", &scale_filter,
                "-q:v", "3",
                "-y",
            ])
            .arg(&out)
            .output()
            .map_err(|e| anyhow!("ffmpeg thumbnail execution failed: {}", e))?;

        if status.status.success() && out.exists() {
            paths.push(out);
        }
    }

    if paths.is_empty() {
        return Err(anyhow!("no preview thumbnails extracted"));
    }
    Ok(paths)
}

/// Ermittle die Dauer einer Videodatei in Sekunden via ffprobe.
pub fn get_duration(video_path: &str) -> Result<f64> {
    let mut cmd = ffprobe_cmd()?;
    let output = cmd
        .args([
            "-v", "error",
            "-show_entries", "format=duration",
            "-of", "csv=p=0",
            video_path,
        ])
        .output()
        .map_err(|e| anyhow!("ffprobe execution failed: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("ffprobe failed: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let duration: f64 = stdout
        .trim()
        .parse()
        .map_err(|e| anyhow!("Could not parse duration '{}': {}", stdout.trim(), e))?;

    Ok(duration)
}

/// Formatiere Sekunden als ffmpeg-kompatiblen Timestamp (HH:MM:SS.mmm).
fn format_timestamp(secs: f64) -> String {
    let total_ms = (secs * 1000.0).round() as u64;
    let hours = total_ms / 3_600_000;
    let minutes = (total_ms % 3_600_000) / 60_000;
    let seconds = (total_ms % 60_000) / 1_000;
    let millis = total_ms % 1_000;
    format!("{:02}:{:02}:{:02}.{:03}", hours, minutes, seconds, millis)
}

/// Prüfe ob ffmpeg verfügbar ist (zuerst App-Verzeichnis, dann PATH).
pub fn check_ffmpeg() -> bool {
    find_ffmpeg().is_some()
}

/// Prüfe ob ffprobe verfügbar ist (zuerst App-Verzeichnis, dann PATH).
pub fn check_ffprobe() -> bool {
    find_ffprobe().is_some()
}

/// Gib den gefundenen ffmpeg-Pfad zurück (für Debugging/Logging).
pub fn ffmpeg_path() -> Option<PathBuf> {
    find_ffmpeg()
}

/// Gib den gefundenen ffprobe-Pfad zurück (für Debugging/Logging).
pub fn ffprobe_path() -> Option<PathBuf> {
    find_ffprobe()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_timestamp() {
        assert_eq!(format_timestamp(0.0), "00:00:00.000");
        assert_eq!(format_timestamp(1.5), "00:00:01.500");
        assert_eq!(format_timestamp(90.0), "00:01:30.000");
        assert_eq!(format_timestamp(3661.789), "01:01:01.789");
    }

    #[test]
    fn test_check_ffmpeg() {
        // Sollte true sein, wenn ffmpeg im PATH ist
        let result = check_ffmpeg();
        // Kein assert — hängt von der Umgebung ab
        eprintln!("ffmpeg verfügbar: {}", result);
    }
}
