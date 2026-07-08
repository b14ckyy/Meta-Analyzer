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
  import VideoRow from './VideoRow.svelte';

  let queueCount = $derived(appState.videoJobs.length);
  let doneCount = $derived(appState.videoJobs.filter((j) => j.status === 'done').length);
  let errorCount = $derived(appState.videoJobs.filter((j) => j.status === 'error').length);
  let pendingApplyCount = $derived(appState.videoJobs.filter((j) => j.status === 'donePending').length);

  let progressPercent = $derived(
    appState.videoProgress.total > 0
      ? Math.round((appState.videoProgress.completed / appState.videoProgress.total) * 100)
      : 0,
  );

  let applyStatus = $state('');

  async function handleApply() {
    applyStatus = 'Writing metadata…';
    try {
      const pendingJobs = appState.videoJobs.filter((j) => j.status === 'donePending');
      const updated = await commands.applyVideoMetadata(
        pendingJobs,
        appState.videoSettings.writeDescription,
        appState.videoSettings.writeGenres,
        appState.videoSettings.writeTags,
        appState.videoSettings.writeTitle,
      );
      for (const j of updated) {
        appState.updateVideoJob(j.id, { status: j.status, errorMsg: j.errorMsg });
      }
      applyStatus = `${updated.filter((j) => j.status === 'done').length} files updated.`;
      setTimeout(() => { applyStatus = ''; }, 3000);
    } catch (error) {
      applyStatus = `Error: ${error}`;
    }
  }
</script>

<div class="h-full flex flex-col bg-base">
  <!-- Header -->
  <div class="shrink-0 px-3 py-2 bg-surface2 border-b border-line shadow-soft flex items-center justify-between">
    <h2 class="text-sm font-semibold text-content">Video Queue</h2>
    <div class="flex items-center gap-2">
      {#if pendingApplyCount > 0 && !appState.isVideoProcessing}
        <button
          onclick={handleApply}
          class="btn btn-primary px-2 py-0.5 text-xs"
        >
          Apply ({pendingApplyCount})
        </button>
      {/if}
      <span class="text-xs text-muted">
        {doneCount}/{queueCount} done
        {#if errorCount > 0}
          <span class="text-danger ml-1">({errorCount} errors)</span>
        {/if}
      </span>
    </div>
  </div>

  {#if applyStatus}
    <div class="shrink-0 px-3 py-1 bg-success/10 border-b border-success/20 text-xs text-success text-center">
      {applyStatus}
    </div>
  {/if}

  <!-- Progress Bar (only during processing) -->
  {#if appState.isVideoProcessing && appState.videoProgress.total > 0}
    <div class="shrink-0 px-3 py-2 bg-accent/10 border-b border-accent/20">
      <div class="flex justify-between text-xs text-muted mb-1">
        <span>Processing videos…</span>
        <span>{appState.videoProgress.completed} / {appState.videoProgress.total}</span>
      </div>
      <progress value={progressPercent} max="100" class="w-full h-1.5"></progress>
    </div>
  {/if}

  <!-- Empty state -->
  {#if queueCount === 0}
    <div class="flex-1 flex items-center justify-center text-muted text-sm">
      <p class="text-center">
        🎬 No videos in queue<br />
        <span class="text-xs">Import a video folder or files to begin</span>
      </p>
    </div>
  {:else}
    <!-- Scrollbare Job-Liste -->
    <div class="flex-1 overflow-y-auto py-0.5">
      {#each appState.videoJobs as job (job.id)}
        <VideoRow {job} />
      {/each}
    </div>
  {/if}
</div>
