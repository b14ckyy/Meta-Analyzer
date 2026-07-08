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
  import { appState } from '../lib/store.svelte';
  import { commands } from '../lib/tauri';

  async function addWithExistingTags(paths: string[]) {
    const existingTagsMap = await commands.readImagesTags(paths);
    appState.addJobs(paths, existingTagsMap);
  }

  /// Fügt einen Video-Job hinzu und lädt asynchron ein Vorschau-Thumbnail nach
  /// (blockiert den Import nicht).
  function addVideoWithThumbnail(path: string, duration: number) {
    const id = appState.addVideoJob(path, [], duration);
    commands
      .createVideoThumbnail(path)
      .then((thumbs) => appState.updateVideoJob(id, { thumbnailPaths: thumbs }))
      .catch(() => {});
  }

  async function handlePickFolder() {
    try {
      const folder = await commands.pickFolder();
      if (folder) {
        if (appState.appMode === 'photo') {
          const files = await commands.scanFolderForImages(folder);
          if (files.length > 0) {
            appState.setImportProgress({ completed: 0, total: files.length });
            await addWithExistingTags(files);
          } else {
            appState.addToast('No images found in the selected folder.', 'info');
          }
        } else {
          const files = await commands.scanFolderForVideos(folder);
          if (files.length > 0) {
            for (const videoPath of files) {
              const job = await commands.createVideoJob(videoPath);
              addVideoWithThumbnail(job.path, job.durationSecs);
            }
          } else {
            appState.addToast('No video files found in the selected folder.', 'info');
          }
        }
      }
    } catch (error) {
      console.error('Failed to pick folder:', error);
      appState.addToast(`Error: ${error}`, 'error');
    }
  }

  async function handlePickFiles() {
    try {
      if (appState.appMode === 'photo') {
        const files = await commands.pickFiles('image');
        if (files.length > 0) {
          appState.setImportProgress({ completed: 0, total: files.length });
          await addWithExistingTags(files);
        }
      } else {
        const files = await commands.pickFiles('video');
        if (files.length > 0) {
          for (const videoPath of files) {
            const job = await commands.createVideoJob(videoPath);
            addVideoWithThumbnail(job.path, job.durationSecs);
          }
        }
      }
    } catch (error) {
      console.error('Failed to pick files:', error);
      appState.addToast(`Error: ${error}`, 'error');
    }
  }

  async function handleClearQueue() {
    const isVideo = appState.appMode === 'video';
    if ((isVideo && appState.isVideoProcessing) || (!isVideo && appState.isProcessing)) {
      appState.addToast('Cannot clear queue while processing.', 'error');
      return;
    }
    const msg = isVideo ? 'Clear all videos from the queue?' : 'Clear all items from the queue?';
    if (!(await appState.confirm(msg))) return;
    if (isVideo) appState.clearVideoQueue();
    else appState.clearQueue();
  }

  function handleClearDone() {
    if (appState.appMode === 'video') appState.clearDoneVideoJobs();
    else appState.clearDoneJobs();
  }

  let clearDoneDisabled = $derived(
    appState.appMode === 'photo'
      ? appState.jobs.filter((j) => j.status === 'done' || j.status === 'skipped').length === 0
      : appState.videoJobs.filter((j) => j.status === 'done' || j.status === 'donePending').length === 0,
  );

  let clearDisabled = $derived(
    appState.appMode === 'photo'
      ? appState.jobs.length === 0 || appState.isProcessing
      : appState.videoJobs.length === 0 || appState.isVideoProcessing,
  );

  let importPercent = $derived(
    appState.importProgress && appState.importProgress.total > 0
      ? Math.round((appState.importProgress.completed / appState.importProgress.total) * 100)
      : 0,
  );
</script>

<div class="p-4 border-b border-line bg-surface">
  <h1 class="font-bold text-lg mb-4 text-content">Meta-Analyzer</h1>
  <div class="flex flex-col gap-2">
    <button
      onclick={handlePickFolder}
      disabled={!!appState.importProgress}
      class="px-4 py-2 bg-accent text-white rounded-lg hover:bg-accent/90 text-sm disabled:bg-accent/50 disabled:cursor-not-allowed"
    >
      📁 Import Folder
    </button>
    <button
      onclick={handlePickFiles}
      disabled={!!appState.importProgress}
      class="px-4 py-2 bg-accent text-white rounded-lg hover:bg-accent/90 text-sm disabled:bg-accent/50 disabled:cursor-not-allowed"
    >
      {appState.appMode === 'photo' ? '📷 Add Images' : '🎬 Add Videos'}
    </button>
    <div class="grid grid-cols-2 gap-2">
      <button
        onclick={handleClearDone}
        disabled={clearDoneDisabled}
        class="px-3 py-2 bg-surface2 text-content border border-line rounded-lg disabled:opacity-50 disabled:cursor-not-allowed hover:bg-base text-sm"
      >
        ✅ Clear Done
      </button>
      <button
        onclick={handleClearQueue}
        disabled={clearDisabled}
        class="px-3 py-2 bg-surface2 text-content border border-line rounded-lg disabled:opacity-50 disabled:cursor-not-allowed hover:bg-base text-sm"
      >
        🗑️ Clear All
      </button>
    </div>
  </div>

  {#if appState.importProgress}
    <div class="mt-3">
      <div class="flex justify-between text-xs text-muted mb-1">
        <span>Reading metadata…</span>
        <span>{appState.importProgress.completed} / {appState.importProgress.total}</span>
      </div>
      <progress value={importPercent} max="100" class="w-full h-1.5"></progress>
    </div>
  {/if}
</div>
