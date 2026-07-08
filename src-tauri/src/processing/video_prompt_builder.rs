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

use crate::video_models::{VideoProfile, VocabularyMode};

/// Video-spezifische Content-Rules (mit genres-Feld)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct VideoContentRules {
    #[serde(default)]
    pub special_rules: Vec<String>,
    #[serde(default)]
    pub genres: Vec<String>,
    #[serde(default)]
    pub allowed_tags: Vec<String>,
    /// Optional description template — überschreibt den generischen OUTPUT FORMAT description-String.
    /// Wird direkt in den Prompt eingefügt.
    #[serde(default)]
    pub description_template: Option<String>,
}

/// Build the full system prompt for video analysis.
///
/// Das LLM wird aufgefordert, JSON mit description, genres und keywords
/// zurückzugeben. Genres werden NUR aus der `content_rules.genres`-Liste gewählt
/// (strict). Keywords folgen dem VocabularyMode.
///
/// Alle Bullet Points sind Single-Line — kein künstliches Word-Wrapping.
pub fn build_video_prompt(
    profile: &VideoProfile,
    vocabulary: &[String],
    content_rules: &VideoContentRules,
) -> String {
    let mut parts: Vec<String> = Vec::new();
    let lang_blocks = crate::processing::prompt_builder::language_blocks(&profile.language);

    // 1 ROLE
    parts.push("You are an expert video tag analyzer for a media library.".into());

    // 2 TASK
    parts.push(format!(
        "TASK: Analyze the video still frame group and create between {} and {} tags.",
        profile.min_tags, profile.max_tags
    ));

    // 3 LANGUAGE (only if non-English) — verbindliche Output-Sprache
    if let Some((early, _)) = &lang_blocks {
        parts.push(early.clone());
    }

    // 4 CONTENT TYPE (only if not General)
    if profile.content_type != "General" {
        parts.push(format!("CONTENT TYPE: {}", profile.content_type));
    }

    // 5 SPECIAL RULES (from content-type rules file)
    if !content_rules.special_rules.is_empty() {
        let mut lines: Vec<String> = content_rules
            .special_rules
            .iter()
            .map(|r| format!("- {}", r))
            .collect();
        lines.insert(0, "SPECIAL RULES:".into());
        parts.push(lines.join("\n"));
    }

    // ── Build merged tag list ──
    let mut all_tags: Vec<String> = Vec::new();

    for tag in &content_rules.allowed_tags {
        let t = tag.trim().to_lowercase();
        if !t.is_empty() && !all_tags.contains(&t) {
            all_tags.push(t);
        }
    }
    for tag in vocabulary {
        let t = tag.trim().to_lowercase();
        if !t.is_empty() && !all_tags.contains(&t) {
            all_tags.push(t);
        }
    }
    for tag in profile.custom_vocabulary.split(',') {
        let t = tag.trim().to_lowercase();
        if !t.is_empty() && !all_tags.contains(&t) {
            all_tags.push(t);
        }
    }

    let has_tags = !all_tags.is_empty();

    // 6 AGGRESSIVE MODE BLOCK (only if tags exist)
    if has_tags {
        let block = match profile.vocabulary_mode {
            VocabularyMode::Strict => {
                "STRICT MODE - VERY IMPORTANT:\n\
                - You MUST ONLY use tags from the ALLOWED TAGS list below.\n\
                - You are NOT allowed to create any new tags.\n\
                - If the list has fewer matching tags than the minimum, output fewer.\n\
                - Do NOT invent tags to reach the minimum count.\n\
                - Violating this rule is not allowed."
                    .into()
            }
            VocabularyMode::Recommended => {
                format!(
                    "RECOMMENDED MODE:\n\
                    - Prefer using tags from the ALLOWED TAGS list below.\n\
                    - Only create your own tags if the list has not enough matching tags for what you see.\n\
                    - If the list covers the content well, do NOT create extra tags.\n\
                    - If you find fewer than {} tags from the list, create reasonable ones yourself if you are confident about the content.",
                    profile.min_tags
                )
            }
            VocabularyMode::Optional => {
                format!(
                    "OPTIONAL MODE:\n\
                    - The ALLOWED TAGS list below is a suggestion.\n\
                    - Use it as inspiration or completely ignore it.\n\
                    - Create between {} and {} tags based on what you actually see.",
                    profile.min_tags, profile.max_tags
                )
            }
        };
        parts.push(block);
    }

    // 7 RULE
    parts.push(
        "RULE:\n\
        - Follow the ALLOWED TAGS MODE strictly.\n\
        - Only create tags for content you can clearly see across the video still frames.\n\
        - If in doubt about the entire video, leave the tag out.\n\
        - Never use vague or speculative tags."
            .into(),
    );

    // 8 GENRES (nur wenn definiert)
    if !content_rules.genres.is_empty() {
        let genre_list = content_rules.genres.join(", ");
        parts.push(format!(
            "GENRES:\n\
            - You MUST choose 2 to 4 genres from this list ONLY:\n\
            {}\n\
            - Do NOT invent genres that are not in this list.\n\
            - These describe the overarching category of the video.",
            genre_list
        ));
    }

    // 9 OUTPUT FORMAT — JSON
    // If the content rules provide a description_template, use it verbatim and let
    // the model follow its tone; otherwise fall back to a neutral generic style.
    // Any category-specific description style therefore lives in its rule file,
    // not in the source.
    let (description_instruction, description_rules) = if let Some(template) = &content_rules.description_template {
        (
            template.as_str(),
            "Follow the description instruction above exactly, matching its tone, style and level of detail.",
        )
    } else {
        (
            "A short 1-2 sentence description of the video content (aim for ~300 characters, max 512).",
            "Describe the scene concisely.",
        )
    };

    let output_format = format!(
        "OUTPUT FORMAT (strict):\n\
        You MUST output ONLY valid JSON with exactly these four fields:\n\
        {{\n\
        \"title\": \"A short descriptive title (3-15 words)\",\n\
        \"description\": \"{}\",\n\
        \"genres\": [\"genre1\", \"genre2\", \"genre3\"],\n\
        \"keywords\": [\"tag1\", \"tag2\", \"tag3\", ...]\n\
        }}\n\
        \n\
        Rules for the fields:\n\
        - \"title\": A concise, descriptive title of 3 to 15 words. Capture the essence of the video like a movie title. Use wording consistent with the description style. Reference the raw filename where applicable — the filename often contains the series name, episode number, scene, or performer names that should be used in the title.\n\
        - \"description\": {}\n\
        - \"genres\": Choose 2 to 4 overarching categories. Follow the GENRES list above if provided.\n\
        - \"keywords\": List all clearly visible details ({min} to {max} tags). Follow the ALLOWED TAGS rules above.\n\
        - Never repeat tags across or within fields.\n\
        - description must be concise but descriptive.",
        description_instruction,
        description_rules,
        min = profile.min_tags,
        max = profile.max_tags,
    );
    parts.push(output_format);

    // 10 IMPORTANT
    parts.push(
        "IMPORTANT:\n\
        Create tags based on what you ACTUALLY SEE across the video still frames.\n\
        Select or create tags based on visual analysis of all combined frames.\n\
        Do NOT copy tags from examples that do not match.\n\
        Accuracy and relevance matter most."
            .into(),
    );

    // 11 CRITICAL
    let lang = if profile.language.is_empty() {
        "English".to_string()
    } else {
        profile.language.clone()
    };
    parts.push(format!(
        "CRITICAL:\n\
        - The response must be valid JSON - no extra text before or after\n\
        - \"keywords\" must be comma-separated values in a JSON array\n\
        - \"title\" must be a single string of 3 to 15 words\n\
        - title, description, genres AND every keyword MUST be written in {0} — translate allowed tags and genres into {0} as needed; this is required and never breaks the ALLOWED TAGS / GENRES rules\n\
        - Never repeat tags\n\
        - Never use tags for content that is not clearly visible\n\
        - Follow the rules above strictly — no exceptions\n\
        - Remember the OUTPUT LANGUAGE, CONTENT TYPE, SPECIAL RULES, GENRES, and ALLOWED TAGS guidelines above",
        lang
    ));

    // 12 ALLOWED TAGS (right before the final instruction) + Übersetzungsregel
    if has_tags {
        parts.push(format!("ALLOWED TAGS: {}", all_tags.join(", ")));
        if let Some((_, translation)) = &lang_blocks {
            parts.push(translation.clone());
        }
    }

    // 13 FINAL
    parts.push("Now analyze the video frames and output ONLY valid JSON:".into());

    parts.join("\n\n")
}

/// Load video content rules from {app_data}/content_rules/{slug}.json.
/// Falls back to embedded default rules if the file doesn't exist.
/// Parst als VideoContentRules (mit genres-Feld).
pub fn load_video_content_rules(app_data_dir: &std::path::Path, slug: &str) -> VideoContentRules {
    let path = app_data_dir
        .join("content_rules")
        .join(format!("{}.json", slug));

    if path.exists() {
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(rules) = serde_json::from_str::<VideoContentRules>(&content) {
                return rules;
            }
        }
    }

    // Embedded fallback. Only the hardcoded defaults (General / Custom) are
    // compiled in; all other video categories load exclusively from disk.
    let embedded = match slug {
        "video_general" => Some(include_str!("../../content_rules/video_general.json")),
        "video_custom" => Some(include_str!("../../content_rules/video_custom.json")),
        _ => None,
    };

    match embedded {
        Some(json) => serde_json::from_str(json).unwrap_or_default(),
        None => VideoContentRules::default(),
    }
}

/// Load a vocabulary file from the app data directory (identisch zu prompt_builder).
pub fn load_vocabulary(app_data_dir: &std::path::Path, slug: &str) -> Vec<String> {
    let path = app_data_dir.join("vocabulary").join(format!("{}.txt", slug));
    if !path.exists() {
        return vec![];
    }
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    content
        .lines()
        .filter(|line| {
            let t = line.trim();
            !t.is_empty() && !t.starts_with('#')
        })
        .map(|line| line.trim().to_string())
        .collect()
}
