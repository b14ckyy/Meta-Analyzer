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
  import { convertFileSrc } from '@tauri-apps/api/core';
  import type { ImageJob } from '../lib/types';
  import { appState } from '../lib/store.svelte';
  import { commands } from '../lib/tauri';
  import EditableChips from './EditableChips.svelte';

  interface Props {
    job: ImageJob;
  }

  let { job }: Props = $props();
  let expanded = $state(false);

  let previewUrl = $derived(expanded ? convertFileSrc(job.path) : '');

  let statusIcon = $derived.by(() => {
    switch (job.status) {
      case 'pending': return '⏳';
      case 'processing': return '🤖';
      case 'done': return '✅';
      case 'donePending': return '⏸️';
      case 'error': return '❌';
      case 'skipped': return '⏭️';
      default: return '•';
    }
  });

  function toggleExpanded() {
    expanded = !expanded;
  }

  function handleRemove() {
    if (job.status === 'processing') return;
    appState.removeJob(job.id);
  }

  async function handleRequeue() {
    if (job.status !== 'done' && job.status !== 'error' && job.status !== 'skipped' && job.status !== 'donePending') return;
    appState.requeueJob(job.id);
    if (!appState.isProcessing) {
      // Start just this one job immediately
      const singleJob = { ...job, status: 'pending' as const, tags: [], existingTags: [], errorMsg: null };
      appState.setProcessing(true);
      appState.setPaused(false);
      try {
        await commands.startProcessing([singleJob], appState.settings, appState.activeProfile);
      } catch {
        appState.setProcessing(false);
      }
    }
  }
</script>

<div class="mx-2 my-1.5 rounded-xl border border-line bg-surface shadow-soft overflow-hidden transition-colors hover:border-accent/40">
  <!-- Header-Zeile: Status + Dateiname + Meta -->
  <div
    onclick={toggleExpanded}
    class="flex items-center gap-2 px-3 py-2.5 hover:bg-surface2 cursor-pointer text-sm"
    role="button"
    tabindex="0"
    onkeydown={(e) => e.key === 'Enter' && toggleExpanded()}
  >
    <span class="w-6 text-center shrink-0">{statusIcon}</span>
    <img
      src={convertFileSrc(job.path)}
      alt=""
      onclick={(e) => { e.stopPropagation(); commands.revealInExplorer(job.path); }}
      title="Show in Explorer"
      class="w-16 h-9 rounded object-cover border border-line shrink-0 bg-base cursor-pointer hover:brightness-110"
      loading="lazy"
      onerror={(e) => { (e.target as HTMLImageElement).style.visibility = 'hidden'; }}
    />
    <span class="flex-1 truncate font-medium text-content">{job.fileName}</span>

    {#if job.status === 'done'}
      <span class="text-xs text-success shrink-0">{job.tags.length} tags</span>
    {:else if job.status === 'donePending'}
      <span class="text-xs text-warning shrink-0">{job.tags.length} tags · pending</span>
    {:else if job.status === 'skipped'}
      <span class="text-xs text-muted shrink-0">skipped</span>
    {:else if job.status === 'error'}
      <span class="text-xs text-danger shrink-0">Error</span>
    {:else if job.status === 'processing'}
      <span class="w-4 h-4 border-2 border-accent border-t-transparent rounded-full animate-spin shrink-0"></span>
    {/if}

    <span class="text-xs text-muted shrink-0">{expanded ? '▲' : '▼'}</span>
  </div>

  <!-- ── EINGEKLAPPT: kompakte Zusammenfassung + Actions ── -->
  {#if !expanded}
    {#if (job.status === 'done' || job.status === 'donePending') && job.tags.length > 0}
      <div class="px-3 pb-2 border-t border-line bg-surface">
        <div class="py-1.5">
          <p class="text-[10px] text-muted font-semibold uppercase tracking-wide mb-0.5">
            Tags ({job.tags.length}){job.status === 'donePending' ? ' · pending apply' : ''}
          </p>
          <p class="text-xs text-content truncate">{job.tags.join(', ')}</p>
        </div>
        <div class="flex gap-1.5 pb-1">
          <button onclick={handleRequeue} class="btn btn-secondary px-1.5 py-0.5 text-[9px] rounded-md">🔄 Re-Queue</button>
          <button onclick={handleRemove} class="btn btn-danger px-1.5 py-0.5 text-[9px] rounded-md">🗑️ Remove</button>
        </div>
      </div>
    {:else if job.status === 'skipped' && job.existingTags.length > 0}
      <div class="px-3 pb-2 border-t border-line bg-surface">
        <div class="py-1.5">
          <p class="text-[10px] text-muted font-semibold uppercase tracking-wide mb-0.5">Existing tags</p>
          <p class="text-xs text-muted truncate">{job.existingTags.join(', ')}</p>
        </div>
      </div>
    {:else if job.status === 'error'}
      <div class="px-3 pb-2 border-t border-line bg-danger/10">
        <p class="text-xs text-danger py-1.5">Parse error — click to expand</p>
        <div class="flex gap-1.5 pb-1">
          <button onclick={handleRequeue} class="btn btn-secondary px-1.5 py-0.5 text-[9px] rounded-md">🔄 Re-Queue</button>
          <button onclick={handleRemove} class="btn btn-danger px-1.5 py-0.5 text-[9px] rounded-md">🗑️ Remove</button>
        </div>
      </div>
    {/if}
  {/if}

  <!-- ── AUSGEKLAPPT: Pfad + Fehler + Tags + Vorschau + Actions ── -->
  {#if expanded}
    <div class="px-3 pb-3 bg-surface2 border-t border-line">
      <p class="text-[10px] text-muted mt-2 mb-2 break-all font-mono">{job.path}</p>

      {#if job.errorMsg}
        <div class="mb-2">
          <p class="text-[10px] text-muted font-semibold uppercase tracking-wide mb-1">Error / Raw model output</p>
          <pre class="text-xs text-danger bg-danger/10 border border-danger/20 rounded p-2 whitespace-pre-wrap break-all max-h-48 overflow-y-auto">{job.errorMsg}</pre>
        </div>
      {/if}

      {#if job.status === 'donePending'}
        <div class="mb-2">
          <p class="text-[10px] text-muted font-semibold uppercase tracking-wide mb-1">Tags ({job.tags.length}) — editable</p>
          <EditableChips
            items={job.tags}
            variant="accent"
            onchange={(t) => appState.updateJob(job.id, { tags: t })}
          />
        </div>
      {:else if job.tags.length > 0}
        <div class="mb-2">
          <p class="text-[10px] text-muted font-semibold uppercase tracking-wide mb-1">Tags ({job.tags.length})</p>
          <div class="flex flex-wrap gap-1">
            {#each job.tags as tag}
              <button onclick={() => appState.copyToClipboard(tag)} title="Copy"
                class="px-2 py-0.5 bg-accent/10 text-accent border border-accent/20 rounded-full text-[10px] hover:border-accent/50 cursor-pointer">{tag}</button>
            {/each}
          </div>
        </div>
      {:else if job.existingTags.length > 0}
        <div class="mb-2">
          <p class="text-[10px] text-muted font-semibold uppercase tracking-wide mb-1">Existing tags ({job.existingTags.length})</p>
          <div class="flex flex-wrap gap-1">
            {#each job.existingTags as tag}
              <span class="px-2 py-0.5 bg-surface2 text-muted border border-line rounded-full text-[10px]">{tag}</span>
            {/each}
          </div>
        </div>
      {/if}

      {#if previewUrl}
        <div class="flex justify-center bg-base rounded-lg border border-line shadow-soft mb-2">
          <img
            src={previewUrl}
            alt={job.fileName}
            class="max-h-64 object-contain"
            loading="lazy"
            onerror={(e) => { (e.target as HTMLImageElement).style.display = 'none'; }}
          />
        </div>
      {/if}

      <!-- Actions -->
      <div class="flex gap-2 mt-1">
        {#if job.status === 'done' || job.status === 'error' || job.status === 'skipped'}
          <button onclick={handleRequeue} class="btn btn-secondary px-2 py-1 text-[10px] rounded-md">🔄 Re-Queue</button>
        {/if}
        {#if job.status !== 'processing'}
          <button onclick={handleRemove} class="btn btn-danger px-2 py-1 text-[10px] rounded-md">🗑️ Remove</button>
        {/if}
      </div>
    </div>
  {/if}
</div>
