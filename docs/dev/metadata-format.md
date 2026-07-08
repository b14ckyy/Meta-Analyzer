# Metadata Format

How tags (and, for videos, title/description/genres) are written back into files.

## Photos — `metadata.rs`

Photo metadata is written at the byte level with `little_exif` (EXIF) plus custom
IPTC and XMP handling, so existing metadata (copyright, camera data, …) is
preserved.

**JPEG** — tags are written to three places for broad reader compatibility:
- **EXIF** `ImageDescription` (comma-separated) and `XPKeywords` (APP1).
- **IPTC** `Keywords` (2:25) in the Photoshop IRB (APP13) — this is what Plex and
  Jellyfin read.
- **XMP** `dc:subject` (APP1) — read by Adobe Bridge / Lightroom and others.

**PNG** — EXIF `ImageDescription` via `little_exif`.

### Reading existing tags

For an "already tagged?" check and to show current tags, the reader tries, in
order:
1. IPTC `Keywords` (APP13 / 2:25) — preferred,
2. EXIF `ImageDescription` — fallback.

## Videos — `video_metadata.rs`

Video metadata is written with **ffmpeg** using **codec copy** (no re-encode), so
it is fast and lossless. ffmpeg cannot edit in place, so a temp file with the same
extension is written and swapped in on success; on failure the original is left
untouched.

Fields come from the model's `VideoMetaOutput { title, description, genres,
keywords }`; each can be toggled independently (write title / description /
genres / tags):

| Field | Written as | Notes |
|-------|-----------|-------|
| `title` | title atom (`©nam` on MP4) | |
| `description` | description atom | |
| `genres` | genre atom | first 3 genres, comma-joined |
| `keywords` | keywords atom | comma-joined tag list |

> The writer deliberately does **not** pass `-movflags use_metadata_tags`. That
> flag stores keys in the `mdta`/`keys` box, which most players ignore. Writing
> the standard keys instead maps them to the classic QuickTime atoms
> (`©nam`/`©gen`/…) that players — and Plex/Jellyfin — actually read.

### Containers

`.mp4`, `.mov`, `.mkv`, `.avi`, `.webm` are accepted. The writer classifies the
container (MP4/QuickTime, Matroska, AVI) and uses the appropriate tag keys. MP4/MOV
are the most reliable; MKV/WebM depend on the codec/muxer.

### JSON fallback

If the model returns something that is not valid JSON, the pipeline falls back to
parsing a comma-separated tag list and writes **keywords only**
(`write_video_metadata_fallback`).

## Plex / Jellyfin

Both read from the written fields:
- `keywords` → tag/keyword field,
- `genre` → genre filter,
- `description` → description/summary.
