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

  let percentage = $derived(
    appState.progress.total > 0
      ? Math.round((appState.progress.completed / appState.progress.total) * 100)
      : 0,
  );

  function formatTime(sec: number): string {
    const m = Math.floor(sec / 60);
    const s = Math.floor(sec % 60);
    return m > 0 ? `${m}m ${s}s` : `${s}s`;
  }
</script>

<div class="p-3 border-b border-line bg-surface2">
  <div class="flex justify-between items-center">
    <div class="flex items-center gap-2 text-xs">
      <span class="font-medium">
        {appState.progress.total === 0
          ? 'Idle'
          : `${appState.progress.completed} / ${appState.progress.total}`}
      </span>
      {#if appState.isProcessing}
        <span class="text-muted">⏱ {formatTime(appState.elapsedSeconds)}</span>
        {#if appState.etaSeconds > 0}
          <span class="text-muted">ETA: {formatTime(appState.etaSeconds)}</span>
        {/if}
        {#if appState.avgSecondsPerJob > 0}
          <span class="text-muted">~{appState.avgSecondsPerJob.toFixed(1)}s/img</span>
        {/if}
      {/if}
    </div>
    <span class="text-xs text-muted">{percentage}%</span>
  </div>
  <progress value={percentage} max="100" class="w-full h-2 mt-1"></progress>
</div>
