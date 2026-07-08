# Prompt Builder

The prompt sent to the LLM is assembled dynamically from a **profile** plus the
selected **content-type rules**. Photos and videos use separate builders
(`prompt_builder.rs`, `video_prompt_builder.rs`) because their output formats
differ: photos return a comma-separated tag list, videos return structured JSON.

## Photo prompt (order)

```
1  ROLE                "You are an expert image tagger for a media library."
2  TASK                "Analyze the image and create between {min} and {max} tags."
2.5 CUSTOM INSTRUCTIONS (only if the profile has custom instructions; takes precedence)
3  OUTPUT LANGUAGE     (only if language != English) — mandatory target language + fallback
4  CONTENT TYPE        (only if not "General")
5  SPECIAL RULES       from the content-type rule file
6  VOCABULARY MODE     Strict | Recommended | Optional (only if a tag list exists)
   TRANSLATION RULE    (only with a non-English language + a tag list)
   ALLOWED TAGS        merged: content rules + vocabulary + custom vocabulary
   FINAL               "Now analyze the image and output only the tags:"
```

## Video prompt (order)

```
1  ROLE                "You are an expert video tag analyzer for a media library."
2  TASK                "Analyze the video still frame group and create {min}-{max} tags."
3  OUTPUT LANGUAGE     (only if language != English)
4  CONTENT TYPE        (only if not "General")
5  SPECIAL RULES       from the content-type rule file
6  VOCABULARY MODE     Strict | Recommended | Optional
7  RULE                only tag what is clearly visible across the still frames
8  GENRES              choose 2–4 from the category's genre pool
9  OUTPUT FORMAT       strict JSON: { title, description, genres, keywords }
   ALLOWED TAGS        merged list
   FINAL               "Now analyze the video frames and output only valid JSON:"
```

The video **custom instructions** (`custom_prompt`) are prepended to the built
prompt by `video_queue.rs`; the photo builder injects them inline (section 2.5),
so they also appear in the photo prompt preview.

## Building blocks

- **Output language** — `language_blocks(language)` returns two directives: an
  early mandatory-language block and a translation rule. English (or empty) adds
  nothing. The translation rule resolves the apparent conflict between strict
  vocabulary and translation: the allowed-tag *set* is constrained, but the
  *language* of the words is the target language, so a translated allowed tag is
  still that allowed tag.
- **Vocabulary mode**
  - **Strict** — use only tags from the allowed list; output fewer than the
    minimum rather than invent tags.
  - **Recommended** — prefer the list; only add your own if it doesn't cover what
    you see.
  - **Optional** — the list is a suggestion; tag freely.
- **Allowed tags** — the union of the content rule's `allowedTags`, the
  vocabulary file, and the profile's custom vocabulary (deduped, lowercased).

## Profiles

```rust
// photo
struct PromptProfile {
    name, min_tags, max_tags, language,
    content_type,            // label, e.g. "General", "Nature & Travel"
    vocabulary_mode,         // Strict | Recommended | Optional
    custom_vocabulary,       // comma-separated extra tags
    custom_prompt,           // free-text instructions (near the top of the prompt)
}

// video: same fields, plus its own custom_prompt handling and genre-aware rules
struct VideoProfile { … same shape … }
```

## Content-type rules

Rule files are JSON in the content-rules folder. Photos and videos are told
apart by filename prefix (`image_*` / `video_*`).

```jsonc
// photo rule (ContentRules)
{
  "label": "Nature & Travel",       // optional display name
  "specialRules": ["…"],            // instructions injected into the prompt
  "allowedTags": ["beach", "…"]     // the tag vocabulary
}

// video rule (VideoContentRules) — adds genres and an optional description style
{
  "label": "Travel & Vacation",
  "specialRules": ["…"],
  "genres": ["Travel", "Vlog", "…"],       // model picks 2–4
  "allowedTags": ["beach", "…"],
  "descriptionTemplate": "…"               // optional; overrides the JSON description style
}
```

**Discovery.** `General` and `Custom` (photo and video) are compiled into the
binary. Every other category is loaded from the content-rules folder at runtime:
`content_types.rs` scans for `image_*` / `video_*` files, takes the display name
from the `label` field (or derives it from the slug), and returns the list to the
dropdowns. So new categories — and optional add-on packs — are just files; no code
change is required.

## Vocabulary files (optional)

```
{data_dir}/vocabulary/{slug}.txt   e.g. image_nature_travel.txt
```

One tag per line; blank lines and `#` comments are ignored. `{data_dir}` is the
portable `data` folder or the per-user app-data dir (see
[architecture.md](architecture.md)).
