# Development Guide

> **Stack:** Svelte 5 + Tauri 2 + Rust · **Build:** Vite (frontend) + Cargo (backend)

## 1. Prerequisites

| Tool | Version | Check | Get it |
|------|---------|-------|--------|
| Node.js | ≥ 18 | `node --version` | [nodejs.org](https://nodejs.org/) |
| npm | ≥ 9 | `npm --version` | (ships with Node) |
| Rust / Cargo | ≥ 1.77 | `rustc --version` | [rustup.rs](https://rustup.rs/) |
| WebView2 | — | pre-installed on Win 10 21H2+ / Win 11 | [Microsoft](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) |
| ffmpeg / ffprobe | — | `ffmpeg -version` | on `PATH` or next to the exe (video only) |
| LM Studio | any | — | [lmstudio.ai](https://lmstudio.ai/) (or any OpenAI-compatible vision server) |

The Tauri CLI is provided through `package.json` (`@tauri-apps/cli`); no global
install is required — use `npx tauri …`.

## 2. First-time setup

```powershell
git clone <repo-url>
cd meta-analyzer
npm install

# sanity check
npm run build                                        # frontend (Vite)
cargo check --manifest-path src-tauri/Cargo.toml     # backend (Rust)

# run with hot-reload
npx tauri dev
```

## 3. Everyday workflow

| Task | Command | Notes |
|------|---------|-------|
| Dev with hot-reload | `npx tauri dev` | Frontend hot-reloads instantly; Rust changes trigger a rebuild. |
| Debug build + console | `.\build-debug.ps1` | Runs the debug exe attached to PowerShell so you see `println!` / `eprintln!` and backtraces. |
| Release (installer + portable) | `.\build-release.ps1` | Produces the NSIS installer and the portable package into `.\release`. |
| Frontend only | `npm run build` | Output to `dist/`. |
| Backend only (release) | `cargo build --release --manifest-path src-tauri/Cargo.toml` | Optimized `meta-analyzer.exe`. |

Because the repo root is a Cargo workspace, `target/` is created at the repo root
(not inside `src-tauri/`).

## 4. Where things live

| What | Path |
|------|------|
| Frontend entry | `src/main.ts` |
| Root component | `src/App.svelte` |
| UI components | `src/components/*.svelte` |
| Global state | `src/lib/store.svelte.ts` |
| Shared types | `src/lib/types.ts` |
| IPC bridge | `src/lib/tauri.ts` |
| Rust entry | `src-tauri/src/main.rs` |
| Commands | `src-tauri/src/commands/*.rs` |
| Processing | `src-tauri/src/processing/*.rs` |
| Content rules | `src-tauri/content_rules/*.json` |

## 5. Adding things

**A Tauri command**
1. Write the function in `src-tauri/src/commands/…`, annotate with `#[tauri::command]`.
2. Register it in the `invoke_handler![...]` list in `main.rs`.
3. Add a wrapper in `src/lib/tauri.ts`.
4. Call it from the store or a component.

**An event**
1. Define a `#[derive(Serialize)]` payload struct (in `models.rs` / `video_models.rs`).
2. Emit it with `app.emit("event-name", payload)?` (`use tauri::Emitter`).
3. Add the TS interface in `types.ts` and a listener in `tauri.ts`.
4. Subscribe in `App.svelte`'s `onMount`.

**A content category**
Drop an `image_*.json` / `video_*.json` rule file into the content-rules folder —
it is discovered at runtime, no code change needed. See
[prompt-builder.md](prompt-builder.md).

## 6. Rust conventions

- **Commands return `Result<T, String>`.** Convert errors with
  `.map_err(|e| e.to_string())` / `format!`.
- **Serde uses camelCase** (`#[serde(rename_all = "camelCase")]`) so the frontend
  always sees/sends camelCase.
- **Emit via `&AppHandle` / `&Window`**, never a global.
- **Shared state** (`ProcessingQueue`, `VideoProcessingQueue`) is registered with
  `.manage()` and pulled into commands via `State<'_, T>`.
- **Data paths** always go through `paths::data_dir(app)` — never
  `app.path().app_data_dir()` directly — so portable mode keeps working.

## 7. Debugging

```powershell
# build-debug.ps1 sets these; or manually:
$env:RUST_LOG = "info,meta_analyzer=debug,tauri=info"
$env:RUST_BACKTRACE = "1"
.\target\debug\meta-analyzer.exe
```

Frontend: `console.log` shows in the WebView console (right-click → Inspect in dev
builds). Release builds have DevTools disabled.

| Symptom | Likely cause / fix |
|---------|--------------------|
| `Failed to fetch models` | LLM server not running / wrong API URL. |
| No tags parsed | Model returned an unexpected shape — try a stronger model. |
| Metadata write failed | File read-only or in use. |
| `No package info in the config file` (build) | A stray `tauri.conf.json` outside `src-tauri/`, or a stale `target/`. |

## 8. Model setup (local)

1. Install LM Studio, load a **vision** model, start its local server (default
   `http://localhost:1234/v1`).
2. In the app, set the API URL, click refresh, pick the model.
3. Reasoning-capable models produce noticeably more reliable JSON. See the README
   for concrete model recommendations.

## 9. Release process

```powershell
# bump version in: src-tauri/Cargo.toml, package.json, src-tauri/tauri.conf.json
.\build-release.ps1
# artifacts land in .\release (installer + portable + zip)
```

There are currently **no automated tests**; verify changes manually (import →
process → apply → check written metadata with e.g. ExifTool / a media player).
