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

//! Dynamic content-type discovery.
//!
//! The app ships a curated set of **SFW** content types (see `CONTENT_TYPES` /
//! `VIDEO_CONTENT_TYPES`). Any additional `*.json` rule file the user drops into
//! `{app_data}/content_rules/` is discovered at runtime and offered as an extra
//! category — this is how optional add-on packs and user/community-shared
//! profiles show up without living in the public repository.
//!
//! Display label resolution for a discovered file: the optional top-level
//! `"label"` field inside the JSON, falling back to a title-cased slug.

use std::path::Path;

use serde::Serialize;

use crate::models::CONTENT_TYPES;
use crate::video_models::VIDEO_CONTENT_TYPES;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentType {
    pub label: String,
    pub slug: String,
}

/// Title-case a slug for display, e.g. `video_travel` -> `Travel`,
/// `my_custom_pack` -> `My Custom Pack`.
fn derive_label(slug: &str) -> String {
    let base = slug.strip_prefix("video_").unwrap_or(slug);
    base.split('_')
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Read the optional top-level `"label"` field from a content-rules JSON file.
fn read_label(path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&content).ok()?;
    value
        .get("label")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Scan `{app_data}/content_rules` for JSON rule files whose slug starts with
/// `prefix` (`image_` for photos, `video_` for videos) and that are NOT part of
/// the built-in defaults. This prefix scheme lets photo and video packs share
/// one folder while staying unambiguous.
fn scan_extra(app_data_dir: &Path, defaults: &[(&str, &str)], prefix: &str) -> Vec<ContentType> {
    let dir = app_data_dir.join("content_rules");
    let default_slugs: Vec<&str> = defaults.iter().map(|(_, s)| *s).collect();
    let mut out: Vec<ContentType> = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&dir) {
        // Stable alphabetical order so the dropdown doesn't reshuffle per run.
        let mut files: Vec<_> = entries.flatten().collect();
        files.sort_by_key(|e| e.file_name());

        for entry in files {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let slug = match path.file_stem().and_then(|s| s.to_str()) {
                Some(s) => s.to_string(),
                None => continue,
            };
            if !slug.starts_with(prefix) {
                continue;
            }
            if default_slugs.contains(&slug.as_str()) {
                continue;
            }
            let label = read_label(&path).unwrap_or_else(|| derive_label(&slug));
            out.push(ContentType { label, slug });
        }
    }

    out
}

/// Full photo content-type list: curated defaults + discovered extras.
pub fn list_photo_content_types(app_data_dir: &Path) -> Vec<ContentType> {
    let mut list: Vec<ContentType> = CONTENT_TYPES
        .iter()
        .map(|(l, s)| ContentType { label: l.to_string(), slug: s.to_string() })
        .collect();
    list.extend(scan_extra(app_data_dir, CONTENT_TYPES, "image_"));
    list
}

/// Full video content-type list: curated defaults + discovered extras.
pub fn list_video_content_types(app_data_dir: &Path) -> Vec<ContentType> {
    let mut list: Vec<ContentType> = VIDEO_CONTENT_TYPES
        .iter()
        .map(|(l, s)| ContentType { label: l.to_string(), slug: s.to_string() })
        .collect();
    list.extend(scan_extra(app_data_dir, VIDEO_CONTENT_TYPES, "video_"));
    list
}

/// Resolve a photo content-type **label** to its rule **slug** (disk-aware).
pub fn photo_slug(app_data_dir: &Path, label: &str) -> String {
    list_photo_content_types(app_data_dir)
        .into_iter()
        .find(|c| c.label == label)
        .map(|c| c.slug)
        .unwrap_or_else(|| "image_general".to_string())
}

/// Resolve a video content-type **label** to its rule **slug** (disk-aware).
pub fn video_slug(app_data_dir: &Path, label: &str) -> String {
    list_video_content_types(app_data_dir)
        .into_iter()
        .find(|c| c.label == label)
        .map(|c| c.slug)
        .unwrap_or_else(|| "video_general".to_string())
}
