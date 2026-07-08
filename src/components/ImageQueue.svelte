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
  import ImageRow from './ImageRow.svelte';

  let queueCount = $derived(appState.jobs.length);
  let doneCount = $derived(appState.jobs.filter((j) => j.status === 'done').length);
  let errorCount = $derived(appState.jobs.filter((j) => j.status === 'error').length);
  let pendingApplyCount = $derived(appState.jobs.filter((j) => j.status === 'donePending').length);

  let applyStatus = $state('');

  async function handleApply() {
    applyStatus = 'Writing metadata…';
    try {
      const pendingJobs = appState.jobs.filter((j) => j.status === 'donePending');
      const updated = await commands.applyPhotoMetadata(pendingJobs);
      for (const j of updated) {
        appState.updateJob(j.id, { status: j.status, errorMsg: j.errorMsg });
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
    <h2 class="text-sm font-semibold text-content">Photo Queue</h2>
    <div class="flex items-center gap-2">
      {#if pendingApplyCount > 0 && !appState.isProcessing}
        <button
          onclick={handleApply}
          class="px-2 py-0.5 bg-success text-white text-xs rounded hover:brightness-110 font-medium"
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

  <!-- Scrollbare Job-Liste -->
  <div class="flex-1 overflow-y-auto py-0.5">
    {#if queueCount === 0}
      <div class="flex-1 flex items-center justify-center h-full text-muted text-sm">
        <p class="text-center">
          📷 No images in queue<br />
          <span class="text-xs">Import an image folder or files to begin</span>
        </p>
      </div>
    {:else}
      {#each appState.jobs as job (job.id)}
        <ImageRow {job} />
      {/each}
    {/if}
  </div>
</div>
