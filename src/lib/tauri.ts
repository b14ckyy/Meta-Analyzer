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

import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type {
  AppSettings,
  ImageJob,
  ImportProgressEvent,
  JobUpdateEvent,
  ProgressEvent,
  ProgressTimerEvent,
  PromptProfile,
  StreamChunkEvent,
  VideoJob,
  VideoSettings,
  VideoProfile,
  VideoJobUpdateEvent,
  VideoProgressEvent,
  VideoFrameExtractedEvent,
} from './types';

export const commands = {
  pickFolder: () => invoke<string | null>('pick_folder'),
  pickFiles: (kind?: 'image' | 'video') => invoke<string[]>('pick_files', { kind }),
  scanFolderForImages: (folder: string) =>
    invoke<string[]>('scan_folder_for_images', { folder }),
  resolveMediaPaths: (paths: string[], kind: 'image' | 'video') =>
    invoke<string[]>('resolve_media_paths', { paths, kind }),
  revealInExplorer: (path: string) => invoke<void>('reveal_in_explorer', { path }),
  readImagesTags: (paths: string[]) =>
    invoke<Record<string, string[]>>('read_images_tags', { paths }),
  fetchAvailableModels: (apiUrl: string, apiKey?: string) =>
    invoke<string[]>('fetch_available_models', { apiUrl, apiKey }),
  startProcessing: (jobs: ImageJob[], settings: AppSettings, profile: PromptProfile) =>
    invoke<void>('start_processing', { jobs, settings, profile }),
  applyPhotoMetadata: (jobs: ImageJob[]) =>
    invoke<ImageJob[]>('apply_photo_metadata', { jobs }),
  pauseProcessing: () => invoke<void>('pause_processing'),
  resumeProcessing: () => invoke<void>('resume_processing'),
  stopProcessing: () => invoke<void>('stop_processing'),
  loadSettings: () => invoke<AppSettings>('load_settings'),
  saveSettings: (settings: AppSettings) =>
    invoke<void>('save_settings', { settings }),

  // Prompt Profiles & Vocabulary
  listProfiles: () => invoke<string[]>('list_profiles'),
  loadProfile: (name: string) => invoke<PromptProfile>('load_profile', { name }),
  saveProfile: (profile: PromptProfile) => invoke<void>('save_profile', { profile }),
  deleteProfile: (name: string) => invoke<void>('delete_profile', { name }),
  loadVocabulary: (contentType: string) => invoke<string[]>('load_vocabulary', { contentType }),
  previewPrompt: (profile: PromptProfile) => invoke<string>('preview_prompt', { profile }),
  listContentTypes: () => invoke<{ label: string; slug: string }[]>('list_content_types'),
  listVideoContentTypes: () =>
    invoke<{ label: string; slug: string }[]>('list_video_content_types'),
  openContentRulesDir: () => invoke<void>('open_content_rules_dir'),

  // ── Video Commands ──
  scanFolderForVideos: (folder: string) =>
    invoke<string[]>('scan_folder_for_videos', { folder }),
  pickVideoFiles: () => invoke<string[]>('pick_files'),
  createVideoJob: (videoPath: string) =>
    invoke<VideoJob>('create_video_job', { videoPath }),
  createVideoThumbnail: (videoPath: string) =>
    invoke<string[]>('create_video_thumbnail', { videoPath }),
  getVideoDuration: (videoPath: string) =>
    invoke<number>('get_video_duration', { videoPath }),
  startVideoProcessing: (
    jobs: VideoJob[],
    settings: AppSettings,
    profile: VideoProfile,
    videoSettings: VideoSettings,
  ) => invoke<void>('start_video_processing', { jobs, settings, profile, videoSettings }),
  stopVideoProcessing: () => invoke<void>('stop_video_processing'),
  pauseVideoProcessing: () => invoke<void>('pause_video_processing'),
  resumeVideoProcessing: () => invoke<void>('resume_video_processing'),
  applyVideoMetadata: (
    jobs: VideoJob[],
    writeDescription: boolean,
    writeGenres: boolean,
    writeTags: boolean,
    writeTitle: boolean,
  ) => invoke<VideoJob[]>('apply_video_metadata', { jobs, writeDescription, writeGenres, writeTags, writeTitle }),

  // ── Video Profiles ──
  listVideoProfiles: () => invoke<string[]>('list_video_profiles'),
  loadVideoProfile: (name: string) => invoke<VideoProfile>('load_video_profile', { name }),
  saveVideoProfile: (profile: VideoProfile) => invoke<void>('save_video_profile', { profile }),
  deleteVideoProfile: (name: string) => invoke<void>('delete_video_profile', { name }),
  loadVideoVocabulary: (videoContentType: string) =>
    invoke<string[]>('load_video_vocabulary', { videoContentType }),
  previewVideoPrompt: (profile: VideoProfile) =>
    invoke<string>('preview_video_prompt', { profile }),
};

export const events = {
  onJobUpdate: (cb: (e: JobUpdateEvent) => void) =>
    listen<JobUpdateEvent>('job-update', ({ payload }) => cb(payload)),
  onProgress: (cb: (e: ProgressEvent) => void) =>
    listen<ProgressEvent>('progress', ({ payload }) => cb(payload)),
  onProgressTimer: (cb: (e: ProgressTimerEvent) => void) =>
    listen<ProgressTimerEvent>('progress-timer', ({ payload }) => cb(payload)),
  onImportProgress: (cb: (e: ImportProgressEvent) => void) =>
    listen<ImportProgressEvent>('import-progress', ({ payload }) => cb(payload)),
  onStreamChunk: (cb: (e: StreamChunkEvent) => void) =>
    listen<StreamChunkEvent>('stream-chunk', ({ payload }) => cb(payload)),

  // ── Video Events ──
  onVideoJobUpdate: (cb: (e: VideoJobUpdateEvent) => void) =>
    listen<VideoJobUpdateEvent>('video-job-update', ({ payload }) => cb(payload)),
  onVideoProgress: (cb: (e: VideoProgressEvent) => void) =>
    listen<VideoProgressEvent>('video-progress', ({ payload }) => cb(payload)),
  onVideoFrameExtracted: (cb: (e: VideoFrameExtractedEvent) => void) =>
    listen<VideoFrameExtractedEvent>('video-frame-extracted', ({ payload }) => cb(payload)),
};
