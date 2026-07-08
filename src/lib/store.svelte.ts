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

import type { ImageJob, JobStatus, AppSettings, ProgressState, WorkerBuffer, PromptProfile, VideoJob, VideoSettings, VideoProfile, AppMode, VideoFrame } from './types';
import { commands } from './tauri';

type Theme = 'light' | 'dark';

/// Wendet das Theme auf das <html>-Element an (Tailwind `darkMode: 'class'`).
function applyTheme(t: Theme) {
  document.documentElement.classList.toggle('dark', t === 'dark');
}

let theme = $state<Theme>('dark');

// ── Toasts & Confirm-Dialog ───────────────────────────────────
export type ToastKind = 'info' | 'success' | 'error';
export interface Toast { id: number; message: string; kind: ToastKind; }
let toasts = $state<Toast[]>([]);
let toastCounter = 0;
let confirmState = $state<{ message: string; resolve: (v: boolean) => void } | null>(null);

// ── Modelle (LM Studio) ───────────────────────────────────────
let availableModels = $state<string[]>([]);
let modelsOnline = $state(false);
let favoriteModels = $state<string[]>([]);

let jobs = $state<ImageJob[]>([]);
let progress = $state<ProgressState>({ completed: 0, total: 0 });
let isProcessing = $state(false);
let isPaused = $state(false);
let settings = $state<AppSettings>(defaultSettings());

// Prompt Profile state
let profileNames = $state<string[]>([]);
let activeProfile = $state<PromptProfile>(defaultProfile());

// Import progress
let importProgress = $state<ProgressState | null>(null);

// Timer / ETA
let elapsedSeconds = $state(0);
let avgSecondsPerJob = $state(0);
let etaSeconds = $state(0);
let timerInterval: ReturnType<typeof setInterval> | null = null;

// Multi-worker live buffers — one entry per actively processing job
let workerBuffers = $state<WorkerBuffer[]>([]);

// Video Timer / ETA
let videoElapsedSeconds = $state(0);
let videoAvgSecondsPerJob = $state(0);
let videoEtaSeconds = $state(0);
let videoTimerInterval: ReturnType<typeof setInterval> | null = null;

// ── Video State ───────────────────────────────────────────────

let appMode = $state<AppMode>('photo');
let videoJobs = $state<VideoJob[]>([]);
let videoProgress = $state<ProgressState>({ completed: 0, total: 0 });
let isVideoProcessing = $state(false);
let isVideoPaused = $state(false);
let videoSettings = $state<VideoSettings>(defaultVideoSettings());
let videoWorkerBuffers = $state<WorkerBuffer[]>([]);
let videoProfileNames = $state<string[]>([]);
let activeVideoProfile = $state<VideoProfile>(defaultVideoProfile());

export const appState = {
  // Theme
  get theme() { return theme; },
  setTheme(t: Theme) {
    theme = t;
    applyTheme(t);
    try { localStorage.setItem('theme', t); } catch {}
  },
  toggleTheme() { this.setTheme(theme === 'dark' ? 'light' : 'dark'); },
  initTheme() {
    let saved: string | null = null;
    try { saved = localStorage.getItem('theme'); } catch {}
    theme = saved === 'light' || saved === 'dark' ? saved : 'dark';
    applyTheme(theme);
  },

  // ── Toasts ──
  get toasts() { return toasts; },
  addToast(message: string, kind: ToastKind = 'info') {
    const id = ++toastCounter;
    toasts = [...toasts, { id, message, kind }];
    setTimeout(() => { toasts = toasts.filter((t) => t.id !== id); }, 3500);
  },
  removeToast(id: number) { toasts = toasts.filter((t) => t.id !== id); },
  async copyToClipboard(text: string) {
    try {
      await navigator.clipboard.writeText(text);
      this.addToast(`Copied: ${text}`, 'success');
    } catch {
      this.addToast('Copy failed', 'error');
    }
  },

  // ── Confirm-Dialog (nicht-blockierend, promise-basiert) ──
  get confirmState() { return confirmState; },
  confirm(message: string): Promise<boolean> {
    return new Promise((resolve) => { confirmState = { message, resolve }; });
  },
  resolveConfirm(v: boolean) {
    confirmState?.resolve(v);
    confirmState = null;
  },

  // ── Modelle ──
  get availableModels() { return availableModels; },
  get modelsOnline() { return modelsOnline; },
  get favoriteModels() { return favoriteModels; },
  /// Fragt die verfügbaren Modelle beim Server ab. Setzt den Online-Status.
  /// Gibt true bei erfolgreichem Kontakt zurück (für das Polling in App.svelte).
  async refreshModels(apiUrl: string): Promise<boolean> {
    try {
      const m = await commands.fetchAvailableModels(apiUrl, settings.apiKey);
      availableModels = m;
      modelsOnline = true;
      // Wenn genau ein Modell verfügbar ist und noch keins gewählt: auto-wählen.
      if (m.length === 1 && !settings.modelName) {
        settings = { ...settings, modelName: m[0] };
      }
      return true;
    } catch {
      modelsOnline = false;
      return false;
    }
  },
  isFavoriteModel(m: string) { return favoriteModels.includes(m); },
  toggleFavoriteModel(m: string) {
    if (!m) return;
    favoriteModels = favoriteModels.includes(m)
      ? favoriteModels.filter((x) => x !== m)
      : [...favoriteModels, m];
    try { localStorage.setItem('favoriteModels', JSON.stringify(favoriteModels)); } catch {}
  },
  initModelFavorites() {
    try {
      const raw = localStorage.getItem('favoriteModels');
      if (raw) {
        const arr = JSON.parse(raw);
        if (Array.isArray(arr)) favoriteModels = arr.filter((x) => typeof x === 'string');
      }
    } catch {}
  },

  // Photo (original properties)
  get jobs() { return jobs; },
  get progress() { return progress; },
  get isProcessing() { return isProcessing; },
  get isPaused() { return isPaused; },
  get settings() { return settings; },
  get importProgress() { return importProgress; },
  get profileNames() { return profileNames; },
  get activeProfile() { return activeProfile; },
  get elapsedSeconds() { return elapsedSeconds; },
  get avgSecondsPerJob() { return avgSecondsPerJob; },
  get etaSeconds() { return etaSeconds; },
  get workerBuffers() { return workerBuffers; },

  // Video
  get appMode() { return appMode; },
  get videoJobs() { return videoJobs; },
  get videoProgress() { return videoProgress; },
  get isVideoProcessing() { return isVideoProcessing; },
  get isVideoPaused() { return isVideoPaused; },
  get videoSettings() { return videoSettings; },
  get videoWorkerBuffers() { return videoWorkerBuffers; },
  get videoProfileNames() { return videoProfileNames; },
  get activeVideoProfile() { return activeVideoProfile; },
  get videoElapsedSeconds() { return videoElapsedSeconds; },
  get videoAvgSecondsPerJob() { return videoAvgSecondsPerJob; },
  get videoEtaSeconds() { return videoEtaSeconds; },

  setAppMode(mode: AppMode) {
    // Kein Leeren der WorkerBuffers hier — sonst verschwindet die Live-Ausgabe eines
    // gerade laufenden Jobs beim Hin- und Herwechseln dauerhaft (Buffers werden von
    // updateJob/updateVideoJob bei processing/done ohnehin verwaltet).
    appMode = mode;
  },

  addJobs(paths: string[], existingTagsMap: Record<string, string[]> = {}) {
    const newJobs: ImageJob[] = paths.map((p) => ({
      id: crypto.randomUUID(),
      path: p,
      fileName: p.split(/[\\/]/).pop() ?? p,
      status: 'pending',
      tags: [],
      existingTags: existingTagsMap[p] ?? [],
      errorMsg: null,
    }));
    jobs = [...jobs, ...newJobs];
    progress = { completed: 0, total: jobs.length };
  },

  requeueJob(jobId: string) {
    jobs = jobs.map((j) =>
      j.id === jobId
        ? { ...j, status: 'pending' as JobStatus, tags: [], existingTags: [], errorMsg: null }
        : j,
    );
  },

  skipTaggedJobs() {
    jobs = jobs.map((j) =>
      j.status === 'pending' && j.existingTags.length > 0
        ? { ...j, status: 'skipped' as JobStatus }
        : j,
    );
  },

  updateJob(jobId: string, update: Partial<ImageJob>) {
    jobs = jobs.map((j) => (j.id === jobId ? { ...j, ...update } : j));

    // Worker buffer management
    if (update.status === 'processing') {
      if (!workerBuffers.find((w) => w.jobId === jobId)) {
        const fileName = jobs.find((j) => j.id === jobId)?.fileName ?? 'Unknown';
        workerBuffers = [...workerBuffers, { jobId, fileName, reasoning: '', content: '', usage: null, tokensPerSecond: 0 }];
      }
    }

    if (update.status === 'done' || update.status === 'error' || update.status === 'skipped') {
      workerBuffers = workerBuffers.filter((w) => w.jobId !== jobId);
    }
  },

  appendStreamChunk(jobId: string, kind: 'reasoning' | 'content' | 'usage', delta: string) {
    workerBuffers = workerBuffers.map((w) => {
      if (w.jobId !== jobId) return w;

      if (kind === 'usage') {
        try {
          const usage = JSON.parse(delta);
          return { ...w, usage, tokensPerSecond: usage.total > 0 ? usage.total / Math.max(1, elapsedSeconds) : 0 };
        } catch {
          return w;
        }
      }

      if (kind === 'reasoning') {
        return { ...w, reasoning: w.reasoning + delta };
      }

      if (kind === 'content') {
        return { ...w, content: w.content + delta };
      }

      return w;
    });
  },

  updateTimer(avgSec: number) {
    avgSecondsPerJob = avgSec;
    const remaining = jobs.filter((j) => j.status === 'pending' || j.status === 'processing').length;
    etaSeconds = avgSec > 0 ? Math.round(avgSec * remaining) : 0;
  },

  // ── Profile helpers ──

  setProfileNames(names: string[]) { profileNames = names; },
  setActiveProfile(p: PromptProfile) { activeProfile = p; },

  updateActiveProfile(p: Partial<PromptProfile>) {
    activeProfile = { ...activeProfile, ...p };
  },

  removeJob(jobId: string) {
    jobs = jobs.filter((j) => j.id !== jobId);
    workerBuffers = workerBuffers.filter((w) => w.jobId !== jobId);
    progress = { completed: progress.completed, total: jobs.length };
  },

  /// Entfernt nur fertige (done/skipped) Foto-Jobs aus der Queue.
  clearDoneJobs() {
    jobs = jobs.filter((j) => j.status !== 'done' && j.status !== 'skipped');
    progress = { completed: 0, total: jobs.length };
  },

  clearQueue() {
    jobs = [];
    progress = { completed: 0, total: 0 };
    importProgress = null;
    workerBuffers = [];
    elapsedSeconds = 0;
    avgSecondsPerJob = 0;
    etaSeconds = 0;
  },

  setProcessing(v: boolean) {
    isProcessing = v;
    if (!v) {
      workerBuffers = [];
      if (timerInterval) { clearInterval(timerInterval); timerInterval = null; }
    } else {
      elapsedSeconds = 0;
      if (timerInterval) clearInterval(timerInterval);
      timerInterval = setInterval(() => { elapsedSeconds++; }, 1000);
    }
  },

  setPaused(v: boolean) { isPaused = v; },
  setProgress(p: ProgressState) { progress = p; },
  setImportProgress(p: ProgressState | null) { importProgress = p; },
  updateSettings(s: Partial<AppSettings>) { settings = { ...settings, ...s }; },

  // ── Video Methods ──

  addVideoJobs(jobs: VideoJob[]) {
    videoJobs = [...videoJobs, ...jobs];
    videoProgress = { completed: 0, total: videoJobs.length };
  },

  addVideoJob(videoPath: string, frames: VideoFrame[] = [], duration: number = 0): string {
    const job: VideoJob = {
      id: crypto.randomUUID(),
      path: videoPath,
      fileName: videoPath.split(/[\\/]/).pop() ?? videoPath,
      durationSecs: duration,
      status: 'pending',
      frames,
      tags: [],
      errorMsg: null,
    };
    videoJobs = [...videoJobs, job];
    videoProgress = { completed: 0, total: videoJobs.length };
    return job.id;
  },

  updateVideoJob(jobId: string, update: Partial<VideoJob>) {
    videoJobs = videoJobs.map((j) => (j.id === jobId ? { ...j, ...update } : j));

    // Video Worker Buffer management
    if (update.status === 'processing') {
      if (!videoWorkerBuffers.find((w) => w.jobId === jobId)) {
        const fileName = videoJobs.find((j) => j.id === jobId)?.fileName ?? 'Unknown';
        videoWorkerBuffers = [...videoWorkerBuffers, { jobId, fileName, reasoning: '', content: '', usage: null, tokensPerSecond: 0 }];
      }
    }

    if (update.status === 'done' || update.status === 'error') {
      videoWorkerBuffers = videoWorkerBuffers.filter((w) => w.jobId !== jobId);
    }
  },

  videoAppendStreamChunk(jobId: string, kind: 'reasoning' | 'content' | 'usage', delta: string) {
    videoWorkerBuffers = videoWorkerBuffers.map((w) => {
      if (w.jobId !== jobId) return w;

      if (kind === 'usage') {
        try {
          const usage = JSON.parse(delta);
          return { ...w, usage, tokensPerSecond: usage.total > 0 ? usage.total / Math.max(1, videoElapsedSeconds) : 0 };
        } catch {
          return w;
        }
      }

      if (kind === 'reasoning') {
        return { ...w, reasoning: w.reasoning + delta };
      }

      if (kind === 'content') {
        return { ...w, content: w.content + delta };
      }

      return w;
    });
  },

  updateVideoFrame(jobId: string, frames: VideoFrame[]) {
    videoJobs = videoJobs.map((j) => (j.id === jobId ? { ...j, frames } : j));
  },

  removeVideoJob(jobId: string) {
    videoJobs = videoJobs.filter((j) => j.id !== jobId);
    videoWorkerBuffers = videoWorkerBuffers.filter((w) => w.jobId !== jobId);
    videoProgress = { completed: 0, total: videoJobs.length };
  },

  clearVideoQueue() {
    videoJobs = [];
    videoProgress = { completed: 0, total: 0 };
    videoWorkerBuffers = [];
  },

  /// Entfernt nur fertige (done/donePending) Video-Jobs aus der Queue.
  clearDoneVideoJobs() {
    videoJobs = videoJobs.filter((j) => j.status !== 'done' && j.status !== 'donePending');
    videoProgress = { completed: 0, total: videoJobs.length };
  },

  videoUpdateTimer(avgSec: number) {
    videoAvgSecondsPerJob = avgSec;
    const remaining = videoJobs.filter((j) => j.status === 'pending' || j.status === 'processing').length;
    videoEtaSeconds = avgSec > 0 ? Math.round(avgSec * remaining) : 0;
  },
  setVideoProgress(p: ProgressState) { videoProgress = p; },
  setVideoPaused(v: boolean) { isVideoPaused = v; },
  setVideoProcessing(v: boolean) {
    isVideoProcessing = v;
    if (!v) {
      isVideoPaused = false;
      videoWorkerBuffers = [];
      if (videoTimerInterval) { clearInterval(videoTimerInterval); videoTimerInterval = null; }
    } else {
      videoElapsedSeconds = 0;
      if (videoTimerInterval) clearInterval(videoTimerInterval);
      videoTimerInterval = setInterval(() => { videoElapsedSeconds++; }, 1000);
    }
  },
  updateVideoSettings(s: Partial<VideoSettings>) {
    videoSettings = { ...videoSettings, ...s };
  },

  // ── Video Profile helpers ──

  setVideoProfileNames(names: string[]) { videoProfileNames = names; },
  setActiveVideoProfile(p: VideoProfile) { activeVideoProfile = p; },
  updateActiveVideoProfile(p: Partial<VideoProfile>) {
    activeVideoProfile = { ...activeVideoProfile, ...p };
  },
};

function defaultSettings(): AppSettings {
  return {
    apiUrl: 'http://localhost:1234',
    modelName: '',
    apiKey: '',
    activeProfile: 'Default',
    maxConcurrent: 1,
    skipTagged: true,
    applyAutomatically: true,
  };
}

function defaultProfile(): PromptProfile {
  return {
    name: 'Default',
    minTags: 10,
    maxTags: 15,
    language: 'English',
    contentType: 'General',
    vocabularyMode: 'Recommended',
    customVocabulary: '',
    customPrompt: '',
  };
}

function defaultVideoSettings(): VideoSettings {
    return {
      numFrames: 20,
      maxConcurrent: 2,
      frameWidth: 1280,
      frameHeight: 720,
      customPrompt: '',
      activeVideoProfile: 'Default',
      writeDescription: true,
      writeGenres: true,
      writeTags: true,
      writeTitle: true,
      applyAutomatically: true,
  };
}

function defaultVideoProfile(): VideoProfile {
  return {
    name: 'Default',
    minTags: 5,
    maxTags: 15,
    language: 'German',
    contentType: 'General',
    vocabularyMode: 'Recommended',
    customVocabulary: '',
    customPrompt: '',
  };
}
