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

use anyhow::Result;
use crate::video_models::VideoMetaOutput;
use super::video_decoder;

/// Write structured video metadata to a video file via ffmpeg (codec copy — no re-encode).
///
/// Fields are written to the standard container tags so normal players (VLC, Plex,
/// Windows) can read them:
///   - title       → ©nam (MP4) / segment title (MKV)
///   - description  → desc / ©des
///   - genre        → ©gen
///   - keywords     → keyw
///   - comment      → combined catch-all of the active fields
///
/// IMPORTANT: we deliberately do NOT pass `-movflags use_metadata_tags`. That flag makes
/// ffmpeg store everything in the freeform `mdta/keys` mechanism, which VLC and most players
/// ignore — the tags are then only visible to ffprobe. Without the flag, ffmpeg maps the
/// standard keys to the classic QuickTime atoms (©nam/©cmt/©gen) that players actually read.
///
/// write_flags steuert, welche Felder geschrieben werden.
pub fn write_video_metadata(
    path: &str,
    output: &VideoMetaOutput,
    write_description: bool,
    write_genres: bool,
    write_tags: bool,
    write_title: bool,
) -> Result<()> {
    let keywords_csv = if write_tags { output.keywords.join(", ") } else { String::new() };
    let genre_str = if write_genres {
        output.genres.iter().take(3).cloned().collect::<Vec<_>>().join(", ")
    } else {
        String::new()
    };
    let desc = if write_description { output.description.as_str() } else { "" };
    let title = if write_title { output.title.as_str() } else { "" };

    write_tags_via_ffmpeg(path, title, desc, &genre_str, &keywords_csv)
}

/// Fallback: schreibt nur comma-separated Tags (für den Fall, dass das LLM kein JSON liefert).
/// Nutzt denselben Schreibpfad wie `write_video_metadata`, nur ohne title/description.
pub fn write_video_metadata_fallback(
    path: &str,
    tags: &[String],
    _write_description: bool,
    write_genres: bool,
    write_tags: bool,
    _write_title: bool,
) -> Result<()> {
    let keywords_csv = if write_tags { tags.join(", ") } else { String::new() };
    let genre_str = if write_genres {
        tags.iter().take(3).cloned().collect::<Vec<_>>().join(", ")
    } else {
        String::new()
    };

    write_tags_via_ffmpeg(path, "", "", &genre_str, &keywords_csv)
}

/// Container-Familie einer Datei-Endung. Bestimmt, welchen Muxer ffmpeg beim Remux nutzt.
enum Container {
    /// ISO-BMFF: mp4/m4v/mov — QuickTime-Atome (©nam …)
    Mp4,
    /// Matroska: mkv/webm — Segment-Title + generische Tags
    Matroska,
    /// AVI: RIFF-INFO-Tags (INAM/ICMT/IGNR …)
    Avi,
}

/// Bestimme Container + die Endung, die die Temp-Datei behalten muss, damit ffmpeg den
/// passenden Muxer wählt. Unbekannte Endungen fallen konservativ auf MP4 zurück.
fn classify(ext: &str) -> (Container, &str) {
    match ext {
        "mp4" | "m4v" | "mov" => (Container::Mp4, ext),
        "mkv" => (Container::Matroska, "mkv"),
        "webm" => (Container::Matroska, "webm"),
        "avi" => (Container::Avi, "avi"),
        _ => (Container::Mp4, "mp4"),
    }
}

/// Schreibt die Metadaten via ffmpeg-Remux. Die Temp-Datei behält die Quell-Endung,
/// damit ffmpeg NIEMALS in ein fremdes Containerformat remuxt (früherer Bug: .avi/.webm
/// wurden als MP4-Stream unter der Original-Endung zurückgeschrieben und dadurch zerstört).
fn write_tags_via_ffmpeg(path: &str, title: &str, description: &str, genre: &str, keywords: &str) -> Result<()> {
    // Wenn nichts zu schreiben ist, nicht unnötig remuxen.
    if title.is_empty() && description.is_empty() && genre.is_empty() && keywords.is_empty() {
        eprintln!("[video_metadata] Nothing to write for {}, skipping", path);
        return Ok(());
    }

    let ffmpeg_path = video_decoder::ffmpeg_path()
        .ok_or_else(|| anyhow::anyhow!("ffmpeg not found — cannot write video metadata"))?;

    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("mp4")
        .to_lowercase();
    let (container, temp_ext) = classify(&ext);

    let temp_path = format!("{}.tmp_meta_analyzer.{}", path, temp_ext);

    let mut cmd = std::process::Command::new(&ffmpeg_path);
    crate::processing::video_decoder::no_window(&mut cmd);
    cmd.arg("-i").arg(path);
    cmd.arg("-codec").arg("copy");
    // Bewusst KEIN `-movflags use_metadata_tags` — siehe Doc-Kommentar an write_video_metadata.

    // Kombinierter Kommentar = alle aktiven Felder (Catch-all für Tools, die nur `comment` lesen).
    let active_parts: Vec<&str> = [title, description, genre, keywords]
        .iter()
        .filter(|s| !s.is_empty())
        .copied()
        .collect();
    let comment = active_parts.join(" | ");

    match container {
        Container::Mp4 | Container::Matroska => {
            add_meta(&mut cmd, "title", title);
            add_meta(&mut cmd, "description", description);
            add_meta(&mut cmd, "genre", genre);
            add_meta(&mut cmd, "keywords", keywords);
            add_meta(&mut cmd, "comment", &comment);
        }
        Container::Avi => {
            // AVI/RIFF unterstützt nur einen kleinen INFO-Tag-Satz zuverlässig.
            // ffmpeg mappt title→INAM, genre→IGNR, comment→ICMT. keywords/description
            // landen daher gebündelt im Kommentar.
            add_meta(&mut cmd, "title", title);
            add_meta(&mut cmd, "genre", genre);
            add_meta(&mut cmd, "comment", &comment);
        }
    }

    cmd.arg("-y");
    cmd.arg(&temp_path);

    let status = cmd
        .output()
        .map_err(|e| anyhow::anyhow!("ffmpeg metadata write failed: {}", e))?;

    if !status.status.success() {
        let stderr = String::from_utf8_lossy(&status.stderr);
        let _ = std::fs::remove_file(&temp_path);
        return Err(anyhow::anyhow!(
            "ffmpeg metadata error: {}",
            stderr.lines().last().unwrap_or("unknown")
        ));
    }

    // Atomar über die Originaldatei ersetzen.
    std::fs::rename(&temp_path, path)?;
    eprintln!("[video_metadata] Metadata written via ffmpeg to {}", path);
    Ok(())
}

/// Hänge ein `-metadata key=value` nur an, wenn value nicht leer ist.
fn add_meta(cmd: &mut std::process::Command, key: &str, value: &str) {
    if !value.is_empty() {
        cmd.arg("-metadata").arg(format!("{}={}", key, value));
    }
}
