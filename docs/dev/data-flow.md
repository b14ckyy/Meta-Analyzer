# Data Flow

Two independent pipelines share the generic worker pool (`processing/pool.rs`).
The frontend starts a run via a command, then reacts to a stream of events.

## Photo pipeline (`queue.rs`)

```
User clicks Start
  └─ commands.startProcessing(jobs, settings, profile)
       └─ ProcessingQueue::start()
            ├─ build the prompt once (shared by all workers)
            └─ worker pool (max_concurrent)
                 └─ per job:
                    ├─ emit "job-update" -> processing
                    ├─ AI call (SSE)  → emit "stream-chunk" (reasoning|content|usage)*
                    ├─ parse + dedupe tags
                    ├─ if apply_automatically: write EXIF/IPTC/XMP
                    │  else:                   keep tags, status "donePending"
                    ├─ emit "job-update" -> done | donePending | error
                    └─ emit "progress" + "progress-timer"
```

When auto-apply is off, results wait as `donePending`; the user edits
title/tags/genres inline and later calls `apply_photo_metadata`, which performs
the actual write.

## Video pipeline (`video_queue.rs`)

```
User clicks Start
  └─ commands.startVideoProcessing(jobs, settings, profile, videoSettings)
       └─ VideoProcessingQueue::start()
            ├─ check ffmpeg/ffprobe
            ├─ build the JSON prompt once
            └─ worker pool (max_concurrent, clamped 1..4)
                 └─ per job, three phases:
                    ├─ 1 EXTRACT   emit "video-job-update" -> extracting
                    │              video_decoder extracts frames -> emit "video-frame-extracted"
                    ├─ 2 ANALYZE   emit "video-job-update" -> processing
                    │              AI call (SSE) → emit "stream-chunk"*  → parse VideoMetaOutput (JSON)
                    │              (one retry on failure; comma-parse fallback if JSON invalid)
                    └─ 3 WRITE     write title/description/genres/keywords via ffmpeg
                                   (or hold as "donePending" for manual apply)
                    └─ emit "video-job-update" -> done | donePending | error
                       emit "video-progress" + "progress-timer"
```

## Events: photo vs. video

| Photo | Video |
|-------|-------|
| `job-update` | `video-job-update` |
| `progress` | `video-progress` |
| `import-progress` | `video-frame-extracted` |
| `progress-timer` (shared) | `progress-timer` (shared) |
| `stream-chunk` (shared) | `stream-chunk` (shared) |

## Notes

1. Separate modules (`queue.rs` / `video_queue.rs`), each with its own pool config.
2. `stream-chunk` is shared; the frontend routes it by `jobId` to the right worker
   buffer (`ThinkingPanel`).
3. `progress-timer` is shared; the frontend routes it by `appMode`.
4. Video has three phases (extract → analyze → write); photo has two
   (analyze → write).
5. Video returns structured JSON (`title`, `description`, `genres`, `keywords`);
   photo returns a comma-separated tag list.
6. Pause/resume/stop are handled inside the pool via a pause control and a
   `CancellationToken`; stopping leaves unprocessed jobs in the queue.
