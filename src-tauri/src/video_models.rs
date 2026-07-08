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

use serde::{Deserialize, Serialize};

// ── Video-spezifische Strukturen ─────────────────────────────────────────
// Ausgelagert aus models.rs für saubere Trennung von Foto/Video.

// ── Video Meta Output (LLM-JSON-Response) ────────────────────────────────

/// Strukturierte LLM-Antwort für Video-Tagging.
/// Das LLM liefert JSON mit description, genres und keywords.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoMetaOutput {
    /// Kurze 1-2 Satz Beschreibung (max 200 Zeichen)
    pub description: String,
    /// 2-4 Hauptkategorien aus dem GENRE-Pool des Profils
    pub genres: Vec<String>,
    /// 8-20 detaillierte Tags (folgt VocabularyMode)
    pub keywords: Vec<String>,
    /// Optional: generierter Titel (separater API-Call)
    #[serde(default)]
    pub title: String,
}

// ── Video Job Status ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum VideoJobStatus {
    Pending,
    Extracting,
    Processing,
    /// Analyse abgeschlossen, wartet auf manuelles Apply
    #[serde(rename = "donePending")]
    DonePending,
    Done,
    Error,
}

// ── Video Frame ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoFrame {
    pub index: u32,
    pub path: String,
    pub timestamp_secs: f64,
}

// ── Video Job ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoJob {
    pub id: String,
    pub path: String,
    pub file_name: String,
    pub duration_secs: f64,
    pub status: VideoJobStatus,
    pub frames: Vec<VideoFrame>,
    pub tags: Vec<String>,
    pub error_msg: Option<String>,
    /// Strukturierte LLM-Ergebnisse, damit sie ein Frontend-Round-Trip (z.B. für
    /// manuelles Apply) überleben. Ohne diese Felder würde serde description/genres/title
    /// beim Deserialisieren verwerfen und das Apply nur die Tags schreiben.
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub genres: Vec<String>,
    #[serde(default)]
    pub title: String,
}

impl VideoJob {
    pub fn new(path: String) -> Self {
        let file_name = std::path::Path::new(&path)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            path,
            file_name,
            duration_secs: 0.0,
            status: VideoJobStatus::Pending,
            frames: vec![],
            tags: vec![],
            error_msg: None,
            description: String::new(),
            genres: vec![],
            title: String::new(),
        }
    }
}

// ── Video Settings ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoSettings {
    pub num_frames: u8,
    pub max_concurrent: u8,
    pub frame_width: u32,
    pub frame_height: u32,
    #[serde(default)]
    pub custom_prompt: String,
    #[serde(default = "default_active_video_profile")]
    pub active_video_profile: String,
    /// In Datei schreiben: Description
    #[serde(default = "default_true")]
    pub write_description: bool,
    /// In Datei schreiben: Genres
    #[serde(default = "default_true")]
    pub write_genres: bool,
    /// In Datei schreiben: Tags/Keywords
    #[serde(default = "default_true")]
    pub write_tags: bool,
    /// In Datei schreiben: Titel (separater API-Call)
    #[serde(default = "default_true")]
    pub write_title: bool,
    /// Automatisch anwenden (ohne manuelles Apply)
    #[serde(default = "default_true")]
    pub apply_automatically: bool,
}

fn default_true() -> bool { true }

impl Default for VideoSettings {
    fn default() -> Self {
        Self {
            num_frames: 20,
            max_concurrent: 2,
            frame_width: 1280,
            frame_height: 720,
            custom_prompt: String::new(),
            active_video_profile: "Default".to_string(),
            write_description: true,
            write_genres: true,
            write_tags: true,
            write_title: true,
            apply_automatically: true,
        }
    }
}

fn default_active_video_profile() -> String {
    "Default".to_string()
}

// ── Video Profile ────────────────────────────────────────────────────────
// Gespeichert in app_data_dir/video_profiles/NAME.json

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoProfile {
    pub name: String,
    #[serde(default = "default_video_min_tags")]
    pub min_tags: u8,
    #[serde(default = "default_video_max_tags")]
    pub max_tags: u8,
    #[serde(default = "default_video_language")]
    pub language: String,
    #[serde(default = "default_video_content_type")]
    pub content_type: String,
    #[serde(default)]
    pub vocabulary_mode: VocabularyMode,
    #[serde(default)]
    pub custom_vocabulary: String,
    /// Custom prompt instructions – wird VOR dem standardisierten Prompt eingefügt
    #[serde(default)]
    pub custom_prompt: String,
}

fn default_video_min_tags() -> u8 { 5 }
fn default_video_max_tags() -> u8 { 15 }
fn default_video_language() -> String { "German".to_string() }
fn default_video_content_type() -> String { "General".to_string() }

impl Default for VideoProfile {
    fn default() -> Self {
        Self {
            name: "Default".to_string(),
            min_tags: 5,
            max_tags: 15,
            language: "German".to_string(),
            content_type: "General".to_string(),
            vocabulary_mode: VocabularyMode::Recommended,
            custom_vocabulary: String::new(),
            custom_prompt: String::new(),
        }
    }
}

// ── Vocabulary Mode (geteilt mit Photo, aber hier eigenständig für Video) ─

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum VocabularyMode {
    Strict,
    Recommended,
    Optional,
}

impl Default for VocabularyMode {
    fn default() -> Self {
        VocabularyMode::Recommended
    }
}

impl std::fmt::Display for VocabularyMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VocabularyMode::Strict => write!(f, "Strict"),
            VocabularyMode::Recommended => write!(f, "Recommended"),
            VocabularyMode::Optional => write!(f, "Optional"),
        }
    }
}

// ── Video Content Types ──────────────────────────────────────────────────

/// Built-in video content types (first = default). Only General and Custom are
/// hardcoded/embedded; every other category ships as an editable `video_*.json`
/// rule file and is discovered dynamically at runtime (see content_types.rs).
pub const VIDEO_CONTENT_TYPES: &[(&str, &str)] = &[
    ("General", "video_general"),
    ("Custom", "video_custom"),
];
// NOTE: All other video categories live as external `video_*.json` rule files in
// {app_data}/content_rules/ and are discovered dynamically. Optional add-on packs
// are dropped into the same folder by the user.

// ── Video Vocabulary (pro Content-Type) ──────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct VideoVocabulary {
    #[serde(default)]
    pub tags_pool: Vec<String>,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub special_rules: Vec<String>,
}

// ── Video Events ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoFrameExtractedEvent {
    pub job_id: String,
    pub total: usize,
    pub completed: usize,
    pub frames: Vec<VideoFrame>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoJobUpdateEvent {
    pub job_id: String,
    pub status: VideoJobStatus,
    pub tags: Vec<String>,
    /// Kurzbeschreibung (aus LLM-JSON)
    #[serde(default)]
    pub description: String,
    /// Genres (aus LLM-JSON)
    #[serde(default)]
    pub genres: Vec<String>,
    /// Generierter Titel (aus separatem API-Call)
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub error_msg: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoProgressEvent {
    pub completed: usize,
    pub total: usize,
}

