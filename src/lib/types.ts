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

export type JobStatus = 'pending' | 'processing' | 'done' | 'error' | 'skipped' | 'donePending';
export type VideoJobStatus = 'pending' | 'extracting' | 'processing' | 'donePending' | 'done' | 'error';
export type AppMode = 'photo' | 'video';

export type VocabularyMode = 'Strict' | 'Recommended' | 'Optional';

export interface ImageJob {
  id: string;
  path: string;
  fileName: string;
  status: JobStatus;
  tags: string[];
  existingTags: string[];
  errorMsg: string | null;
}

export interface AppSettings {
  apiUrl: string;
  modelName: string;
  /** Optional API key for cloud providers (OpenAI-compatible, e.g. xAI). Empty = no auth header. */
  apiKey: string;
  activeProfile: string;
  maxConcurrent: number;
  skipTagged: boolean;
  applyAutomatically: boolean;
  videoSettings?: VideoSettings;
}

export interface PromptProfile {
  name: string;
  minTags: number;
  maxTags: number;
  language: string;
  contentType: string;
  vocabularyMode: VocabularyMode;
  customVocabulary: string;
  /** Free-text custom instructions, inserted near the start of the prompt. */
  customPrompt: string;
}

export interface ProgressState {
  completed: number;
  total: number;
}

export interface JobUpdateEvent {
  jobId: string;
  status: JobStatus;
  tags: string[];
  errorMsg: string | null;
}

export interface ProgressEvent {
  completed: number;
  total: number;
}

export interface StreamChunkEvent {
  jobId: string;
  kind: 'reasoning' | 'content' | 'usage';
  delta: string;
}

export interface UsageInfo {
  prompt: number;
  completion: number;
  total: number;
}

export interface WorkerBuffer {
  jobId: string;
  fileName: string;
  reasoning: string;
  content: string;
  usage: UsageInfo | null;
  tokensPerSecond: number;
}

export interface ProgressTimerEvent {
  startTime: number;
  avgSecondsPerJob: number;
}

export interface ImportProgressEvent {
  completed: number;
  total: number;
}

// ── Video Types ───────────────────────────────────────────────

export interface VideoFrame {
  index: number;
  path: string;
  timestampSecs: number;
}

export interface VideoJob {
  id: string;
  path: string;
  fileName: string;
  durationSecs: number;
  status: VideoJobStatus;
  frames: VideoFrame[];
  tags: string[];
  errorMsg: string | null;
  existingTags?: string[];
  /** Kurzbeschreibung aus LLM-JSON */
  description?: string;
  /** Genres aus LLM-JSON */
  genres?: string[];
  /** Generierter Titel (separater API-Call) */
  title?: string;
  /** Vorschau-Thumbnails (eingebettetes Cover oder 5 verteilte Frames), bis die volle Extraktion läuft */
  thumbnailPaths?: string[];
}

export interface VideoSettings {
  numFrames: number;
  maxConcurrent: number;
  frameWidth: number;
  frameHeight: number;
  customPrompt: string;
  activeVideoProfile: string;
  /** Description in Datei schreiben */
  writeDescription: boolean;
  /** Genres in Datei schreiben */
  writeGenres: boolean;
  /** Tags in Datei schreiben */
  writeTags: boolean;
  /** Titel in Datei schreiben */
  writeTitle: boolean;
  /** Automatisch anwenden (ohne manuelles Apply) */
  applyAutomatically: boolean;
}

export interface VideoProfile {
  name: string;
  minTags: number;
  maxTags: number;
  language: string;
  contentType: string;
  vocabularyMode: VocabularyMode;
  customVocabulary: string;
  customPrompt: string;
}

export interface VideoProgressEvent {
  completed: number;
  total: number;
}

export interface VideoFrameExtractedEvent {
  jobId: string;
  total: number;
  completed: number;
  frames: VideoFrame[];
}

export interface VideoJobUpdateEvent {
  jobId: string;
  status: VideoJobStatus;
  tags: string[];
  description?: string;
  genres?: string[];
  title?: string;
  errorMsg: string | null;
}
