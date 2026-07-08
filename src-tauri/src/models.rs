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

// ── Existing enums / structs ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum JobStatus {
    Pending,
    Processing,
    Done,
    Error,
    Skipped,
    /// Analyse fertig, wartet auf manuelles Apply (wenn applyAutomatically aus ist).
    #[serde(rename = "donePending")]
    DonePending,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageJob {
    pub id: String,
    pub path: String,
    pub file_name: String,
    pub status: JobStatus,
    pub tags: Vec<String>,
    pub error_msg: Option<String>,
}

impl ImageJob {
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
            status: JobStatus::Pending,
            tags: vec![],
            error_msg: None,
        }
    }
}

// ── Prompt Profile ───────────────────────────────────────────────────────

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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptProfile {
    pub name: String,
    #[serde(default = "default_min_tags")]
    pub min_tags: u8,
    #[serde(default = "default_max_tags")]
    pub max_tags: u8,
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default = "default_content_type")]
    pub content_type: String,
    #[serde(default)]
    pub vocabulary_mode: VocabularyMode,
    #[serde(default)]
    pub custom_vocabulary: String,
    /// Free-text custom instructions, inserted near the start of the prompt.
    #[serde(default)]
    pub custom_prompt: String,
}

fn default_min_tags() -> u8 { 10 }
fn default_max_tags() -> u8 { 15 }
fn default_language() -> String { "English".to_string() }
fn default_content_type() -> String { "General".to_string() }

impl Default for PromptProfile {
    fn default() -> Self {
        Self {
            name: "Default".to_string(),
            min_tags: 10,
            max_tags: 15,
            language: "English".to_string(),
            content_type: "General".to_string(),
            vocabulary_mode: VocabularyMode::Recommended,
            custom_vocabulary: String::new(),
            custom_prompt: String::new(),
        }
    }
}

/// Built-in photo content types (first = default). Only General and Custom are
/// hardcoded/embedded; every other category ships as an editable `image_*.json`
/// rule file and is discovered dynamically at runtime (see content_types.rs).
pub const CONTENT_TYPES: &[(&str, &str)] = &[
    ("General", "image_general"),
    ("Custom", "image_custom"),
];
// NOTE: All other photo categories live as external `image_*.json` rule files in
// {app_data}/content_rules/ and are discovered dynamically. Optional add-on packs
// are dropped into the same folder by the user.

// ── App Settings ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub api_url: String,
    pub model_name: String,
    /// Optionaler API-Key für Cloud-Provider (OpenAI-kompatibel, z.B. xAI).
    /// Leer = kein Auth-Header (lokale Server wie LM Studio brauchen keinen).
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub active_profile: String,
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent: u8,
    #[serde(default = "default_skip_tagged")]
    pub skip_tagged: bool,
    #[serde(default = "default_apply_automatically")]
    pub apply_automatically: bool,
    #[serde(default)]
    pub video_settings: Option<crate::video_models::VideoSettings>,
}

fn default_max_concurrent() -> u8 { 1 }
fn default_skip_tagged() -> bool { true }
fn default_apply_automatically() -> bool { true }

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            api_url: "http://localhost:1234".to_string(),
            model_name: String::new(),
            api_key: String::new(),
            active_profile: "Default".to_string(),
            max_concurrent: 1,
            skip_tagged: true,
            apply_automatically: true,
            video_settings: None,
        }
    }
}

// ── Content Rules (per content type — Foto) ──────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ContentRules {
    #[serde(default)]
    pub special_rules: Vec<String>,
    #[serde(default)]
    pub allowed_tags: Vec<String>,
}

// ── Events (Photo + generisch) ───────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobUpdateEvent {
    pub job_id: String,
    pub status: JobStatus,
    pub tags: Vec<String>,
    pub error_msg: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressEvent {
    pub completed: usize,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamChunkEvent {
    pub job_id: String,
    pub kind: String,
    pub delta: String,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct UsageInfo {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressTimerEvent {
    pub start_time: f64,
    pub avg_seconds_per_job: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportProgressEvent {
    pub completed: usize,
    pub total: usize,
}
