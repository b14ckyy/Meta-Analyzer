# Content Packs

Meta-Analyzer builds **General** and **Custom** into the app. Every other
content category is an editable JSON rule file that lives in the app's
content-rules directory and is **discovered dynamically at runtime** — drop a
`*.json` rule file in and it appears in the *Content Type* dropdown, no rebuild
required. This lets you edit the shipped categories, add your own, and share
packs with others.

## Where the rule files live

```
%APPDATA%\com.meta-analyzer.app\content_rules\
```

The app seeds the default categories here on first run. Use the **Rules folder**
button next to the *Content Type* dropdown to open it directly.

## Rule file format

A rule file is a JSON document with these fields:

| Field                 | Type       | Purpose                                              |
| --------------------- | ---------- | ---------------------------------------------------- |
| `label`               | string     | Display name in the dropdown (optional; else derived from the file name) |
| `specialRules`        | string[]   | Extra instructions injected into the prompt          |
| `allowedTags`         | string[]   | The tag vocabulary the model may choose from         |
| `genres`              | string[]   | *(video only)* genre pool the model picks 2–4 from   |
| `descriptionTemplate` | string     | *(optional)* overrides the description style          |

Rule files are matched by a **filename prefix** so photo and video packs can
share one folder:

- **Photo** rules must be prefixed with `image_` (e.g. `image_my_pack.json`).
- **Video** rules must be prefixed with `video_` (e.g. `video_my_pack.json`).

(`General` and `Custom` are built into the app and are not files here.)

## Adding a pack

Obtain or author the rule files, drop them into the folder above, and restart
the app. The new categories appear in the dropdown automatically. To keep local
pack files out of version control, place them under `content-packs/local/`
(git-ignored) or simply keep them in the app's content-rules folder.
