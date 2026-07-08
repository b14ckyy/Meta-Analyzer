<!--
  Meta-Analyzer - AI-powered metadata tagger for photos and videos.
  Copyright (C) 2026 b14ckyy

  This program is free software: you can redistribute it and/or modify
  it under the terms of the GNU General Public License as published by
  the Free Software Foundation, either version 3 of the License, or
  (at your option) any later version.

  This program is distributed in the hope that it will be useful,
  but WITHOUT ANY WARRANTY; without even the implied warranty of
  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
  GNU General Public License for more details.

  You should have received a copy of the GNU General Public License
  along with this program.  If not, see <https://www.gnu.org/licenses/>.
-->

<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { getCurrentWebview } from '@tauri-apps/api/webview';
  import { appState } from './lib/store.svelte';
  import { events, commands } from './lib/tauri';
  import FilePicker from './components/FilePicker.svelte';
  import ProgressBar from './components/ProgressBar.svelte';
  import ControlBar from './components/ControlBar.svelte';
  import ImageQueue from './components/ImageQueue.svelte';
  import VideoQueue from './components/VideoQueue.svelte';
  import SettingsPanel from './components/SettingsPanel.svelte';
  import ThinkingPanel from './components/ThinkingPanel.svelte';
  import ModeToggle from './components/ModeToggle.svelte';
  import Overlays from './components/Overlays.svelte';

  type ThinkingMode = 'hidden' | 'small' | 'expanded';
  let thinkingMode = $state<ThinkingMode>('expanded');

  let unlistenJob: (() => void) | undefined;
  let unlistenProgress: (() => void) | undefined;
  let unlistenProgressTimer: (() => void) | undefined;
  let unlistenStream: (() => void) | undefined;
  let unlistenImportProgress: (() => void) | undefined;
  let unlistenVideoJob: (() => void) | undefined;
  let unlistenVideoProgress: (() => void) | undefined;
  let unlistenVideoFrameExtracted: (() => void) | undefined;
  let unlistenVideoStream: (() => void) | undefined;
  let modelPollTimer: ReturnType<typeof setTimeout> | undefined;
  let unlistenDrop: (() => void) | undefined;
  let dragOver = $state(false);

  /// Verarbeitet per Drag & Drop fallengelassene Dateien/Ordner je nach Modus.
  async function handleDrop(paths: string[]) {
    if (paths.length === 0) return;
    const isVideo = appState.appMode === 'video';
    try {
      const files = await commands.resolveMediaPaths(paths, isVideo ? 'video' : 'image');
      if (files.length === 0) {
        appState.addToast('No matching files in drop', 'info');
        return;
      }
      if (isVideo) {
        for (const vp of files) {
          const job = await commands.createVideoJob(vp);
          const id = appState.addVideoJob(job.path, [], job.durationSecs);
          commands.createVideoThumbnail(vp)
            .then((t) => appState.updateVideoJob(id, { thumbnailPaths: t }))
            .catch(() => {});
        }
      } else {
        appState.setImportProgress({ completed: 0, total: files.length });
        const map = await commands.readImagesTags(files);
        appState.addJobs(files, map);
      }
      appState.addToast(`Imported ${files.length} file(s)`, 'success');
    } catch (e) {
      appState.addToast(`Import failed: ${e}`, 'error');
    }
  }

  /// Prüft periodisch, ob der LM-Studio-Server erreichbar ist, und lädt die Modellliste.
  /// Bis zum ersten Kontakt wird schnell gepollt (4s), danach seltener (30s), um die
  /// Liste aktuell zu halten — ohne dass man beim App-Start manuell refreshen muss.
  async function pollModels() {
    const ok = await appState.refreshModels(appState.settings.apiUrl);
    modelPollTimer = setTimeout(pollModels, ok ? 30_000 : 4_000);
  }

  onMount(async () => {
    console.log('[App] Setting up event listeners...');
    appState.initModelFavorites();
    unlistenJob = await events.onJobUpdate((e) => {
      console.log('[App] job-update:', e.status, e.jobId.slice(0, 8));
      appState.updateJob(e.jobId, {
        status: e.status,
        tags: e.tags,
        errorMsg: e.errorMsg,
      });
    });

    unlistenProgress = await events.onProgress((e) => {
      appState.setProgress(e);
      if (e.total > 0 && e.completed >= e.total) {
        appState.setProcessing(false);
      }
    });

    unlistenProgressTimer = await events.onProgressTimer((e) => {
      if (appState.appMode === 'video') {
        appState.videoUpdateTimer(e.avgSecondsPerJob);
      } else {
        appState.updateTimer(e.avgSecondsPerJob);
      }
    });

    unlistenImportProgress = await events.onImportProgress((e) => {
      appState.setImportProgress({ completed: e.completed, total: e.total });
      if (e.completed >= e.total) {
        setTimeout(() => appState.setImportProgress(null), 1500);
      }
    });

    unlistenStream = await events.onStreamChunk((e) => {
      // Photo jobs only — video jobs are handled by unlistenVideoStream
      if (!appState.videoJobs.find((j) => j.id === e.jobId)) {
        appState.appendStreamChunk(e.jobId, e.kind, e.delta);
      }
    });

    // ── Video Events ──
    unlistenVideoJob = await events.onVideoJobUpdate((e) => {
      console.log('[App] video-job-update:', e.status, e.jobId.slice(0, 8));
      appState.updateVideoJob(e.jobId, {
        status: e.status,
        tags: e.tags,
        description: e.description,
        genres: e.genres,
        title: e.title,
        errorMsg: e.errorMsg,
      });
    });

    unlistenVideoProgress = await events.onVideoProgress((e) => {
      appState.setVideoProgress(e);
      if (e.total > 0 && e.completed >= e.total) {
        appState.setVideoProcessing(false);
      }
    });

    unlistenVideoFrameExtracted = await events.onVideoFrameExtracted((e) => {
      console.log('[App] video-frame-extracted:', e.jobId.slice(0, 8), e.completed, '/', e.total);
      // Frames in die VideoJob-Daten schreiben
      if (e.frames && e.frames.length > 0) {
        appState.updateVideoJob(e.jobId, { frames: e.frames });
      }
    });

    // Video-Stream-Chunks auch an die Video-Buffer weiterleiten
    // (derselbe stream-chunk Event — unterscheidet sich nur durch jobId)
    unlistenVideoStream = await events.onStreamChunk((e) => {
      // Prüfen ob die jobId zu einem Video-Job gehört
      if (appState.videoJobs.find((j) => j.id === e.jobId)) {
        appState.videoAppendStreamChunk(e.jobId, e.kind, e.delta);
      }
    });

    console.log('[App] Event listeners ready');

    // Load settings
    try {
      const saved = await commands.loadSettings();
      appState.updateSettings(saved);
      // Video-Settings aus den globalen Settings wiederherstellen
      if (saved.videoSettings) {
        appState.updateVideoSettings(saved.videoSettings);
      }
    } catch (error) {
      console.error('Failed to load settings:', error);
    }

    // Load profiles
    try {
      const names = await commands.listProfiles();
      appState.setProfileNames(names);
      if (names.length > 0) {
        const profile = await commands.loadProfile(names[0]);
        appState.setActiveProfile(profile);
      }
    } catch (error) {
      console.error('Failed to load profiles:', error);
    }

    // Load video profiles
    try {
      const videoNames = await commands.listVideoProfiles();
      appState.setVideoProfileNames(videoNames);
      if (videoNames.length > 0) {
        const vp = await commands.loadVideoProfile(videoNames[0]);
        appState.setActiveVideoProfile(vp);
      }
    } catch (error) {
      console.error('Failed to load video profiles:', error);
    }

    // Modell-Watcher starten (nutzt die geladene apiUrl).
    pollModels();

    // Drag & Drop einrichten.
    try {
      unlistenDrop = await getCurrentWebview().onDragDropEvent((ev) => {
        const p = ev.payload;
        if (p.type === 'enter' || p.type === 'over') {
          dragOver = true;
        } else if (p.type === 'leave') {
          dragOver = false;
        } else if (p.type === 'drop') {
          dragOver = false;
          handleDrop(p.paths ?? []);
        }
      });
    } catch (e) {
      console.error('Failed to set up drag & drop:', e);
    }
  });

  onDestroy(() => {
    if (modelPollTimer) clearTimeout(modelPollTimer);
    unlistenDrop?.();
    unlistenJob?.();
    unlistenProgress?.();
    unlistenProgressTimer?.();
    unlistenImportProgress?.();
    unlistenStream?.();
    unlistenVideoJob?.();
    unlistenVideoProgress?.();
    unlistenVideoFrameExtracted?.();
    unlistenVideoStream?.();
  });

  // Thinking panel height class based on mode
  let thinkingHeightClass = $derived(
    thinkingMode === 'expanded' ? 'h-1/2' : thinkingMode === 'small' ? 'h-40' : 'h-auto',
  );
</script>

<svelte:head>
  <title>Meta-Analyzer</title>
</svelte:head>

<div class="h-screen flex flex-row bg-base text-content overflow-hidden">
  <!-- Left panel: 50% width -->
  <div class="w-1/2 flex flex-col border-r border-line shrink-0 h-screen bg-base">
    <ModeToggle />
    <FilePicker />
    <ProgressBar />
    <ControlBar />
    <div class="flex-1 min-h-0 flex flex-col">
      <SettingsPanel />
    </div>
  </div>

  <!-- Right panel: queue on top, thinking on bottom -->
  <div class="flex-1 flex flex-col overflow-hidden">
    <div class="flex-1 min-h-0 overflow-hidden">
      {#if appState.appMode === 'video'}
        <VideoQueue />
      {:else}
        <ImageQueue />
      {/if}
    </div>
    <div class="{thinkingHeightClass} shrink-0">
      <ThinkingPanel mode={thinkingMode} onModeChange={(m) => (thinkingMode = m)} />
    </div>
  </div>
</div>

{#if dragOver}
  <div class="fixed inset-0 z-40 pointer-events-none flex items-center justify-center bg-accent/10 border-4 border-dashed border-accent">
    <div class="px-4 py-2 rounded-xl bg-surface text-content shadow-card text-sm font-medium">
      {appState.appMode === 'video' ? '🎬 Drop videos here' : '📷 Drop images here'}
    </div>
  </div>
{/if}

<Overlays />
