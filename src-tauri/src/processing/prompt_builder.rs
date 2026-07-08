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

use crate::models::{PromptProfile, VocabularyMode, ContentRules};

/// Baut die Sprach-Direktiven für eine Zielsprache.
///
/// Gibt `(early_directive, translation_rule)` zurück, oder `None`, wenn Englisch bzw.
/// leer (dann keine Sprach-Anweisung nötig). Die `translation_rule` löst den scheinbaren
/// Widerspruch zwischen STRICT-Mode und Übersetzung auf: Ein übersetzter erlaubter Tag
/// ist immer noch derselbe erlaubte Tag — die Regeln beschränken die MENGE der Begriffe,
/// nicht die SPRACHE der Wörter. Der Aufrufer entscheidet anhand von `has_tags`, ob die
/// `translation_rule` (die sich auf ALLOWED TAGS bezieht) eingefügt wird.
pub fn language_blocks(language: &str) -> Option<(String, String)> {
    let lang = language.trim();
    if lang.is_empty() || lang.eq_ignore_ascii_case("english") {
        return None;
    }

    let early = format!(
        "OUTPUT LANGUAGE — MANDATORY:\n\
        - Write ALL final output in {lang}: every tag/keyword, and — if present — the title, the genres, and the description.\n\
        - Do NOT output words in English (or any other language) unless it is a proper name that has no {lang} equivalent.\n\
        - FALLBACK: If you are not fluent in {lang} and cannot produce it reliably, output everything in English instead. Never produce broken, garbled, or transliterated {lang} — correct English is better than wrong {lang}.",
        lang = lang
    );

    let translation = format!(
        "TRANSLATION RULE (read carefully — this resolves an apparent conflict):\n\
        - The ALLOWED TAGS list (and the GENRES list, if present) may be written in English. You MUST still output every tag, genre, title and description in {lang}.\n\
        - Workflow: (1) match what you see against the ALLOWED TAGS / GENRES in their original language, then (2) TRANSLATE each selected tag and genre into {lang} for your final answer.\n\
        - A translated allowed tag or genre is STILL that same allowed item. Translating it is REQUIRED and does NOT count as inventing a new one — it does NOT violate STRICT MODE, the ALLOWED TAGS MODE, or the GENRES constraint.\n\
        - The rules restrict only the SET of concepts you may use (defined by ALLOWED TAGS / GENRES), NOT the language of the words: concept = constrained, language = {lang}.\n\
        - Never output an allowed tag or genre in English when {lang} is requested. Never add concepts that are not covered by the ALLOWED TAGS / GENRES (unless the mode explicitly allows it).",
        lang = lang
    );

    Some((early, translation))
}

/// Build the full system prompt for the LLM based on the profile.
/// Set `is_video = true` for video mode (different role, task, and context text).
/// ALLOWED TAGS is placed last (just before the final instruction)
/// so the model sees it most recently — closer to the output.
/// All bullet points are single lines — no artificial word-wrapping.
pub fn build_prompt(profile: &PromptProfile, vocabulary: &[String], content_rules: &ContentRules, is_video: bool) -> String {
    let mut parts: Vec<String> = Vec::new();
    let lang_blocks = language_blocks(&profile.language);

    // 1 ROLE
    if is_video {
        parts.push("You are an expert video tag analyzer for a media library.".into());
    } else {
        parts.push("You are an expert image tagger for a media library.".into());
    }

    // 2 TASK
    if is_video {
        parts.push(format!(
            "TASK: Analyze the video still frame group and create between {} and {} tags.",
            profile.min_tags, profile.max_tags
        ));
    } else {
        parts.push(format!(
            "TASK: Analyze the image and create between {} and {} tags.",
            profile.min_tags, profile.max_tags
        ));
    }

    // 2.5 CUSTOM INSTRUCTIONS (user-provided; take precedence over everything else)
    if !profile.custom_prompt.trim().is_empty() {
        parts.push(format!(
            "CUSTOM INSTRUCTIONS (these take precedence — follow them above all else):\n{}",
            profile.custom_prompt.trim()
        ));
    }

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

    // 6 AGGRESSIVE MODE BLOCK (only if tags exist — identisch für Photo/Video)
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

    // 7 RULE (leicht angepasst für Video)
    if is_video {
        parts.push(
            "RULE:\n\
            - Follow the ALLOWED TAGS MODE strictly.\n\
            - Only create tags for content you can clearly see across the video still frames.\n\
            - If in doubt about the entire video, leave the tag out.\n\
            - Never use vague or speculative tags.
            - Follow the SPECIAL RULES strictly, especially regarding video content analysis and relevance."
                .into(),
        );
    } else {
        parts.push(
            "RULE:\n\
            - Follow the ALLOWED TAGS MODE strictly.\n\
            - Only create tags for content you can clearly see.\n\
            - If in doubt, leave the tag out.\n\
            - Never use vague or speculative tags."
                .into(),
        );
    }

    // 8 OUTPUT FORMAT
    parts.push(
        "OUTPUT FORMAT (strict):\n\
        - Only comma-separated tags, nothing else."
            .into(),
    );

    // 9 IMPORTANT
    if is_video {
        parts.push(
            "IMPORTANT:\n\
            Create tags based on what you ACTUALLY SEE across the video still frames.\n\
            Select or create tags based on visual analysis of all combined frames.\n\
            Do NOT copy tags from examples that do not match.\n\
            Accuracy and relevance matter most."
                .into(),
        );
    } else {
        parts.push(
            "IMPORTANT:\n\
            Create tags based on what you ACTUALLY SEE in the image.\n\
            Do NOT copy tags from examples that do not match.\n\
            Accuracy and relevance matter most."
                .into(),
        );
    }

    // 10 CRITICAL
    let lang = if profile.language.is_empty() { "English".to_string() } else { profile.language.clone() };
    parts.push(format!(
        "CRITICAL:\n\
        - All tags must be comma-separated\n\
        - Every tag MUST be written in {0} — translate allowed tags into {0} as needed; this is required and never breaks the ALLOWED TAGS rules\n\
        - Never repeat tags\n\
        - Never use tags for content that is not clearly visible\n\
        - Follow the rules above strictly — no exceptions\n\
        - Remember the OUTPUT LANGUAGE, CONTENT TYPE, SPECIAL RULES, and ALLOWED TAGS guidelines above",
        lang
    ));

    // 11 ALLOWED TAGS (right before the final instruction) + Übersetzungsregel
    if has_tags {
        parts.push(format!("ALLOWED TAGS: {}", all_tags.join(", ")));
        if let Some((_, translation)) = &lang_blocks {
            parts.push(translation.clone());
        }
    }

    // 12 FINAL
    if is_video {
        parts.push("Now analyze the video frames and output only the tags:".into());
    } else {
        parts.push("Now analyze the image and output only the tags:".into());
    }

    parts.join("\n\n")
}

/// Load a vocabulary file from the app data directory.
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

/// Get the vocabulary file path for a content type label.
pub fn vocabulary_path(app_data_dir: &std::path::Path, content_type: &str) -> std::path::PathBuf {
    let slug = crate::content_types::photo_slug(app_data_dir, content_type);
    app_data_dir.join("vocabulary").join(format!("{}.txt", slug))
}

/// Load content rules from {app_data}/content_rules/{slug}.json.
/// Falls back to embedded default rules if the file doesn't exist.
pub fn load_content_rules(app_data_dir: &std::path::Path, slug: &str) -> ContentRules {
    let path = app_data_dir
        .join("content_rules")
        .join(format!("{}.json", slug));

    if path.exists() {
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(rules) = serde_json::from_str::<ContentRules>(&content) {
                return rules;
            }
        }
    }

    // Embedded fallback. Only the hardcoded defaults (General / Custom) are
    // compiled in; all other categories load exclusively from disk.
    let embedded = match slug {
        "image_general" => Some(include_str!("../../content_rules/image_general.json")),
        "image_custom" => Some(include_str!("../../content_rules/image_custom.json")),
        "video_general" => Some(include_str!("../../content_rules/video_general.json")),
        "video_custom" => Some(include_str!("../../content_rules/video_custom.json")),
        _ => None,
    };

    match embedded {
        Some(json) => serde_json::from_str(json).unwrap_or_default(),
        None => ContentRules::default(),
    }
}
