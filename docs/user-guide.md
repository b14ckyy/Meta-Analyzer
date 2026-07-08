# Meta-Analyzer — User Guide

Meta-Analyzer looks at your photos and videos with a local AI model and writes
descriptive tags (and, for videos, a title, description and genres) straight into
the files — so your library becomes searchable in Plex, Jellyfin, digiKam or your
OS file search. Everything runs on your machine.

---

## 1. What you need

- **Windows 10 (21H2+) / 11** with the **WebView2** runtime (already present on
  up-to-date systems).
- A **local AI model server** that speaks the OpenAI API and can *see images* —
  easiest is [LM Studio](https://lmstudio.ai/) with a **vision** model loaded.
  A reasoning-capable model gives noticeably better results (see the project
  README for concrete model suggestions).
- **ffmpeg** on your system — only needed for **video** tagging. Not needed for
  photos.

## 2. Get the app

Two editions, same program:

- **Installer** (`…_x64-setup.exe`) — installs normally; settings are stored in
  your Windows user profile.
- **Portable** (`Meta-Analyzer-Portable`) — unzip and run `Meta-Analyzer.exe`.
  Everything is kept in the `data` folder next to the exe, so it leaves no trace
  and you can carry it on a USB stick. Keep the exe and the `data` folder
  together.

## 3. First run

1. Start LM Studio (or your server), load a vision model, and start its local
   server (default `http://localhost:1234/v1`).
2. Open Meta-Analyzer → **Settings**.
3. Enter the **API URL**, then click the refresh button to load models and pick
   yours. (For a private/cloud server you can also set an optional API key.)
4. Choose a **Content Type** (e.g. *General*, *Nature & Travel*, *Parties &
   Events*). This tunes which tags the model prefers.

## 4. Tag photos

1. Switch to **Photo** mode.
2. **Add files or a folder** (or drag & drop). Only image files are accepted.
3. Click **Start**. Each image streams its analysis live; you can **pause/stop**
   anytime.
4. **Review & apply:**
   - With *Auto-Apply* on, tags are written immediately.
   - With it off, results wait for you: edit the tags inline (remove a tag by
     clicking it, add with **+**), then **Apply** to write them into the file.

Tags are written to EXIF/IPTC/XMP, which Plex, Jellyfin, Lightroom, etc. read.

## 5. Tag videos

1. Switch to **Video** mode (needs ffmpeg).
2. Add video files. The app samples frames from each clip and analyzes them
   together.
3. Besides tags, videos also get a **title**, a **description** and **genres** —
   you can choose which of these get written (Settings).
4. Review/edit and **Apply** just like photos. Metadata is written with ffmpeg
   without re-encoding, so it's fast and lossless.

## 6. Profiles, content types & packs

- A **profile** bundles your settings (tag count, language, vocabulary mode,
  custom instructions, custom vocabulary). Save several and switch between them.
- **Vocabulary mode** controls how strictly the model sticks to the category's
  tag list: *Strict* (only that list), *Recommended* (prefer it), *Optional*
  (free).
- **Custom Instructions** let you steer the model in your own words.
- **Content categories** beyond *General* / *Custom* are editable files. Use the
  **Rules folder** button next to *Content Type* to open the folder; drop in your
  own category files (or optional add-on packs) and they appear in the dropdown
  automatically.
- **Optional profile packs** are available in the
  [extra_profiles](extra_profiles/) folder — download one, unzip it, and drop the
  files into your content-rules folder (via the **Rules folder** button).

## 7. Language

Set a target **Language** in the profile and all output (tags, title, genres,
description) is produced in that language. If the model isn't fluent in it, the
app falls back to English rather than producing broken text.

## 8. Where your data lives

- **Portable:** the `data` folder next to `Meta-Analyzer.exe`.
- **Installed:** `%APPDATA%\com.meta-analyzer.app`.

Both hold your settings, profiles and the editable content-rule files.
