# Architecture

> **Stack:** Tauri 2 (Rust) · Svelte 5 (TypeScript) · OpenAI-compatible vision LLM (LM Studio)
> **State:** Svelte 5 runes — no external state library
> **Metadata:** EXIF/IPTC/XMP (photos) · MP4/MKV atoms via ffmpeg (videos)

Meta-Analyzer has two parallel pipelines — **photo** and **video** — that share one
engine (AI client, worker pool, prompt/content-rule system). The frontend is a
single-page Svelte app; all heavy work happens in the Rust backend and is driven
by Tauri commands and events.

---

## 1. Project layout

```
meta-analyzer/
├── src/                              # Frontend (Svelte 5 + TypeScript)
│   ├── App.svelte                    # Root component, wires events -> store
│   ├── lib/
│   │   ├── store.svelte.ts           # Global reactive state (appState)
│   │   ├── tauri.ts                  # Command + event bridge to Rust
│   │   └── types.ts                  # Shared TypeScript types
│   └── components/                   # FilePicker, ControlBar, ModeToggle,
│                                     # Image/VideoQueue + Row, SettingsPanel,
│                                     # Prompt/VideoPromptBuilder, ThinkingPanel,
│                                     # EditableChips, Overlays, ...
│
├── src-tauri/                        # Backend (Rust)
│   ├── content_rules/                # Content-type rule files (JSON)
│   │   ├── general.json, custom.json         # embedded photo defaults
│   │   ├── image_*.json                       # external photo categories (seeded)
│   │   ├── video_general.json, video_custom.json  # embedded video defaults
│   │   └── video_*.json                       # external video categories (seeded)
│   └── src/
│       ├── main.rs                   # App setup, content-rule seeding
│       ├── paths.rs                  # data_dir() — portable vs. installed
│       ├── content_types.rs          # dynamic content-type discovery
│       ├── models.rs                 # photo types + events
│       ├── video_models.rs           # video types + events
│       ├── window_state.rs           # window size persistence
│       ├── commands/                 # Tauri command handlers
│       │   ├── dialog.rs             # file dialogs, scans, model list
│       │   ├── processor.rs          # photo start/pause/resume/stop, apply
│       │   ├── video_processor.rs    # video start/stop/pause, thumbnails, apply
│       │   └── settings.rs           # settings + profiles, content-type list, preview
│       └── processing/
│           ├── pool.rs               # generic worker pool (shared)
│           ├── queue.rs              # photo pipeline
│           ├── video_queue.rs        # video pipeline (extract -> analyze -> write)
│           ├── ai_client.rs          # OpenAI-compatible SSE streaming client
│           ├── prompt_builder.rs     # photo prompt + content rules + vocabulary
│           ├── video_prompt_builder.rs # video prompt (JSON output)
│           ├── metadata.rs           # EXIF/IPTC/XMP writer (photos)
│           ├── video_metadata.rs     # ffmpeg metadata writer (videos)
│           └── video_decoder.rs      # frame/thumbnail extraction (ffmpeg/ffprobe)
│
├── build-release.ps1                 # installer + portable release into .\release
├── build-debug.ps1                   # debug build attached to the console
└── docs/dev/                         # this documentation
```

The repo root is a small Cargo **workspace** whose only member is `src-tauri`. It
exists to hold the shared `[profile.release]` optimization flags and keep the
build `target/` at the root.

---

## 2. Frontend component tree

```
App.svelte
├── ModeToggle            # photo <-> video
├── FilePicker            # add files/folders (filtered by mode)
├── ProgressBar           # progress + ETA
├── ControlBar            # start / pause / stop
├── SettingsPanel
│   └── PromptBuilder / VideoPromptBuilder
├── ImageQueue / VideoQueue
│   └── ImageRow / VideoRow  (inline edit via EditableChips)
├── ThinkingPanel         # per-worker live token stream
└── Overlays              # toasts + confirm dialog
```

State lives in `store.svelte.ts` as `$state` runes, split into a photo set
(`jobs`, `workerBuffers`, `activeProfile`, …) and a video set (`videoJobs`,
`videoWorkerBuffers`, `activeVideoProfile`, …). `App.svelte` subscribes to Tauri
events on mount and funnels them into the store.

---

## 3. Backend modules

### Processing
| Module | Responsibility |
|--------|----------------|
| `pool.rs` | Generic worker pool: concurrency, pause control, cancellation, per-job outcome. Shared by both pipelines. |
| `queue.rs` | Photo pipeline: build prompt once, run jobs through the pool, write metadata (or hold for manual apply). |
| `video_queue.rs` | Video pipeline: extract frames -> AI analyze -> write metadata. Adds pause support. |
| `ai_client.rs` | OpenAI-compatible `/v1/chat/completions` SSE streaming, shared by both modes. Optional Bearer API key. |
| `prompt_builder.rs` | Photo prompt assembly, language directives, content-rule + vocabulary loading. |
| `video_prompt_builder.rs` | Video prompt assembly with strict JSON output (title/description/genres/keywords). |
| `metadata.rs` | EXIF/IPTC/XMP read + write for JPEG/PNG. |
| `video_metadata.rs` | Metadata write for MP4/MKV/AVI via ffmpeg (codec copy). |
| `video_decoder.rs` | Keyframe/thumbnail extraction via ffmpeg/ffprobe. |

### Commands
| Module | Commands (selection) |
|--------|----------------------|
| `dialog.rs` | `pick_folder`, `pick_files`, `scan_folder_for_images`, `resolve_media_paths`, `read_images_tags`, `fetch_available_models`, `reveal_in_explorer` |
| `processor.rs` | `start_processing`, `pause_processing`, `resume_processing`, `stop_processing`, `apply_photo_metadata` |
| `video_processor.rs` | `start/stop/pause/resume_video_processing`, `create_video_thumbnail`, `get_video_duration`, `apply_video_metadata` |
| `settings.rs` | `load/save_settings`, photo+video profile CRUD, `list_content_types` / `list_video_content_types`, `load_vocabulary`, `preview_prompt`, `open_content_rules_dir` |

---

## 4. Content types

`General` and `Custom` are compiled into the binary; every other category ships
as an editable JSON rule file and is discovered at runtime (see
[prompt-builder.md](prompt-builder.md)). Photo rule files use an `image_` prefix,
video rule files a `video_` prefix, so both can live in one folder. Optional
add-on packs are simply extra rule files dropped into that folder.

Data (settings, profiles, content_rules, vocabulary) lives in a `data` folder
next to the executable in **portable** mode, or in the per-user app-data dir when
installed — resolved once by `paths::data_dir()`.

---

## 5. Key dependencies (`src-tauri/Cargo.toml`)

| Crate | Use |
|-------|-----|
| `tauri` (+ `tauri-plugin-dialog`) | IPC, events, windows, file dialogs |
| `reqwest` (rustls, stream) | HTTP client for the LLM API (SSE) |
| `serde` / `serde_json` | JSON serialization across the IPC boundary |
| `tokio` / `tokio-util` | async runtime, semaphores, `CancellationToken` |
| `image` | image decode/encode helpers |
| `little_exif` + `img-parts` | EXIF/IPTC/XMP writing for photos |
| `base64` | inline image encoding for the vision API |
| `uuid` | job IDs |
| `anyhow` | error handling |

`ffmpeg`/`ffprobe` are **external** binaries (looked up next to the executable,
then on `PATH`); they are not bundled.
