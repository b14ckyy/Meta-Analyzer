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
  import type { VideoJob } from '../lib/types';
  import { appState } from '../lib/store.svelte';
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { invoke } from '@tauri-apps/api/core';
  import { commands } from '../lib/tauri';
  import EditableChips from './EditableChips.svelte';

  let { job }: { job: VideoJob } = $props();

  let expanded = $state(false);

  let statusLabel = $derived.by(() => {
    switch (job.status) {
      case 'pending': return '⏳';
      case 'extracting': return '📥';
      case 'processing': return '🤖';
      case 'donePending': return '⏸️';
      case 'done': return '✅';
      case 'error': return '❌';
    }
  });

  let durationFormatted = $derived(
    job.durationSecs > 0
      ? `${Math.floor(job.durationSecs / 60)}:${String(Math.floor(job.durationSecs % 60)).padStart(2, '0')}`
      : '--:--',
  );

  // Vorschauquelle fürs Header-Thumbnail: erster extrahierter Frame, sonst das
  // mittlere Import-Thumbnail (repräsentativer als Anfang/Ende).
  let previewSrc = $derived.by(() => {
    if (job.frames.length > 0) return frameSrc(job.frames[0].path);
    const t = job.thumbnailPaths;
    if (t && t.length > 0) return frameSrc(t[Math.floor(t.length / 2)]);
    return '';
  });

  // Pfad zur Reasoning-Datei aus dem ersten Frame ableiten
  let reasoningPath = $derived.by(() => {
    if (job.frames.length > 0) {
      const framePath = job.frames[0].path;
      const sep = framePath.includes('\\') ? '\\' : '/';
      const dir = framePath.substring(0, framePath.lastIndexOf(sep));
      const result = `${dir}${sep}${job.id}.reasoning.txt`;
      console.log('[VideoRow] reasoningPath:', result, '| job.id:', job.id, '| framePath:', framePath);
      return result;
    }
    return null;
  });

  async function openReasoning() {
    const p = reasoningPath;
    if (p) {
      try {
        await invoke('open_file', { path: p });
      } catch (e) {
        console.error('[VideoRow] Failed to open reasoning file:', e);
        try {
          // Fallback
          window.open(convertFileSrc(p), '_blank');
        } catch {}
      }
    }
  }

  function toggleExpanded() {
    expanded = !expanded;
  }

  function handleRemove() {
    if (job.status === 'extracting' || job.status === 'processing') return;
    appState.removeVideoJob(job.id);
  }

  function handleRequeue() {
    if (job.status !== 'done' && job.status !== 'error') return;
    appState.updateVideoJob(job.id, { status: 'pending', tags: [], errorMsg: null });
  }

  // Konvertiere Tauri-Dateipfad in einen Asset-URL fürs img-Tag
  function frameSrc(path: string): string {
    try {
      return convertFileSrc(path);
    } catch {
      return path;
    }
  }
</script>

<div class="mx-2 my-1.5 rounded-xl border border-line bg-surface shadow-soft overflow-hidden transition-colors hover:border-accent/40">
  <!-- Header-Zeile: Dateiname + Status + Dauer -->
  <div
    onclick={toggleExpanded}
    class="flex items-center gap-2 px-3 py-2.5 hover:bg-surface2 cursor-pointer text-sm"
    role="button"
    tabindex="0"
    onkeydown={(e) => e.key === 'Enter' && toggleExpanded()}
  >
    <span class="w-6 text-center shrink-0">{statusLabel}</span>
    {#if previewSrc}
      <img src={previewSrc} alt=""
        onclick={(e) => { e.stopPropagation(); commands.revealInExplorer(job.path); }}
        title="Show in Explorer"
        class="w-16 h-9 rounded object-cover border border-line shrink-0 bg-base cursor-pointer hover:brightness-110" loading="lazy" />
    {/if}
    <span class="flex-1 truncate font-medium text-content">{job.fileName}</span>
    <span class="text-xs text-muted shrink-0">{durationFormatted}</span>

    {#if job.status === 'pending'}
      <span class="text-xs text-accent font-medium shrink-0">{job.frames.length || '?'} frames</span>
    {:else if job.status === 'done' || job.status === 'donePending'}
      <span class="text-xs text-success shrink-0">{job.tags.length} keywords</span>
    {:else if job.status === 'error'}
      <span class="text-xs text-danger shrink-0">Error</span>
    {:else if job.status === 'processing' || job.status === 'extracting'}
      <span class="w-4 h-4 border-2 border-accent border-t-transparent rounded-full animate-spin shrink-0"></span>
    {/if}

    <span class="text-xs text-muted shrink-0">{expanded ? '▲' : '▼'}</span>
  </div>

  <!-- ── EINGEKLAPPT: Description + Genres + Actions ── -->
  {#if !expanded}
    {#if job.status === 'done' && (job.description || (job.genres && job.genres.length > 0) || job.title)}
      <div class="px-3 pb-2 border-t border-line bg-surface">
        <div class="flex items-start gap-3 py-1.5">
          <!-- Title -->
          {#if job.title}
            <div class="flex-1 min-w-0">
              <p class="text-[10px] text-muted font-semibold uppercase tracking-wide mb-0.5">Title</p>
              <p class="text-xs text-content truncate font-medium">{job.title}</p>
            </div>
          {/if}
          <!-- Description (einzellig, abgeschnitten) -->
          {#if job.description}
            <div class="flex-1 min-w-0">
              <p class="text-[10px] text-muted font-semibold uppercase tracking-wide mb-0.5">Description</p>
              <p class="text-xs text-content truncate">{job.description}</p>
            </div>
          {/if}
          <!-- Genres -->
          {#if job.genres && job.genres.length > 0}
            <div class="shrink-0 max-w-[200px]">
              <p class="text-[10px] text-muted font-semibold uppercase tracking-wide mb-0.5">Genres</p>
              <div class="flex flex-wrap gap-0.5">
                {#each job.genres as genre}
                  <span class="px-2 py-0.5 bg-accent/10 text-accent border border-accent/20 rounded-full text-[9px] font-medium">{genre}</span>
                {/each}
              </div>
            </div>
          {/if}
        </div>
        <!-- Actions (direkt unter Description/Genres) -->
        <div class="flex gap-1.5 pb-1">
          {#if reasoningPath}
            <button onclick={openReasoning} class="btn btn-warning px-1.5 py-0.5 text-[9px] rounded-md">
              🧠 Reasoning
            </button>
          {/if}
          <button onclick={handleRequeue} class="btn btn-secondary px-1.5 py-0.5 text-[9px] rounded-md">
            🔄 Re-Queue
          </button>
          <button onclick={handleRemove} class="btn btn-danger px-1.5 py-0.5 text-[9px] rounded-md">
            🗑️ Remove
          </button>
        </div>
      </div>
    {:else if job.status === 'donePending' && (job.description || (job.genres && job.genres.length > 0) || job.title)}
      <div class="px-3 pb-2 border-t border-line bg-warning/10">
        <div class="flex items-start gap-3 py-1.5">
          {#if job.title}
            <div class="flex-1 min-w-0">
              <p class="text-[10px] text-muted font-semibold uppercase tracking-wide mb-0.5">Title</p>
              <p class="text-xs text-content truncate font-medium">{job.title}</p>
            </div>
          {/if}
          {#if job.description}
            <div class="flex-1 min-w-0">
              <p class="text-[10px] text-muted font-semibold uppercase tracking-wide mb-0.5">Description</p>
              <p class="text-xs text-content truncate">{job.description}</p>
            </div>
          {/if}
          {#if job.genres && job.genres.length > 0}
            <div class="shrink-0 max-w-[200px]">
              <p class="text-[10px] text-muted font-semibold uppercase tracking-wide mb-0.5">Genres</p>
              <div class="flex flex-wrap gap-0.5">
                {#each job.genres as genre}
                  <span class="px-2 py-0.5 bg-accent/10 text-accent border border-accent/20 rounded-full text-[9px] font-medium">{genre}</span>
                {/each}
              </div>
            </div>
          {/if}
        </div>
        <div class="flex gap-1.5 pb-1">
          {#if reasoningPath}
            <button onclick={openReasoning} class="btn btn-warning px-1.5 py-0.5 text-[9px] rounded-md">
              🧠 Reasoning
            </button>
          {/if}
          <button onclick={handleRequeue} class="btn btn-secondary px-1.5 py-0.5 text-[9px] rounded-md">
            🔄 Re-Queue
          </button>
          <button onclick={handleRemove} class="btn btn-danger px-1.5 py-0.5 text-[9px] rounded-md">
            🗑️ Remove
          </button>
        </div>
      </div>
    {:else if job.status === 'error'}
      <div class="px-3 pb-2 border-t border-line bg-danger/10">
        <div class="flex gap-1.5 py-1">
          {#if reasoningPath}
            <button onclick={openReasoning} class="btn btn-warning px-1.5 py-0.5 text-[9px] rounded-md">
              🧠 Reasoning
            </button>
          {/if}
          <button onclick={handleRequeue} class="btn btn-secondary px-1.5 py-0.5 text-[9px] rounded-md">
            🔄 Re-Queue
          </button>
          <button onclick={handleRemove} class="btn btn-danger px-1.5 py-0.5 text-[9px] rounded-md">
            🗑️ Remove
          </button>
        </div>
      </div>
    {/if}
  {/if}

  <!-- ── AUSGEKLAPPT: Filmstreifen + Keywords + Fehler + Actions ── -->
  {#if expanded}
    <div class="px-3 pb-3 bg-surface2 border-t border-line">
      <!-- Fehler (nur sichtbar aufgeklappt) -->
      {#if job.errorMsg}
        <div class="mb-2 p-2 bg-danger/10 border border-danger/20 rounded text-xs text-danger whitespace-pre-wrap">
          {job.errorMsg}
        </div>
      {/if}

      <!-- Filmstreifen -->
      {#if job.frames.length > 0}
        <div class="overflow-x-auto overflow-y-hidden mb-2" style="max-height: 150px;">
          <div class="flex gap-1.5 p-1" style="width: max-content;">
            {#each job.frames as frame}
              <div class="relative shrink-0 group">
                <img src={frameSrc(frame.path)} alt="Frame {frame.index}"
                  class="h-28 w-auto rounded border border-line object-cover" style="max-height: 120px;" />
                <div class="absolute bottom-0 left-0 right-0 bg-black/60 text-white text-[9px] px-1 py-0.5 text-center rounded-b">
                  {Math.floor(frame.timestampSecs / 60)}:{String(Math.floor(frame.timestampSecs % 60)).padStart(2, '0')}
                </div>
              </div>
            {/each}
          </div>
        </div>
      {:else if job.thumbnailPaths && job.thumbnailPaths.length > 0}
        <!-- Vorschau-Thumbnails, bis die Frames extrahiert sind -->
        <div class="mb-2">
          <div class="flex gap-1.5 overflow-x-auto p-1" style="width: max-content; max-width: 100%;">
            {#each job.thumbnailPaths as t}
              <img src={frameSrc(t)} alt="Preview"
                class="h-28 w-auto rounded border border-line object-cover shrink-0" style="max-height: 120px;" />
            {/each}
          </div>
          {#if job.status === 'extracting'}
            <p class="text-xs text-accent italic mt-1">Extracting frames…</p>
          {/if}
        </div>
      {:else if job.status === 'extracting'}
        <div class="text-xs text-accent italic mb-2">Extracting frames…</div>
      {/if}

      <!-- Description (ausgeklappt voller Text) -->
      {#if job.status === 'donePending'}
        <div class="mb-2">
          <p class="text-[10px] text-muted font-semibold uppercase tracking-wide mb-1">Description — editable</p>
          <textarea
            value={job.description ?? ''}
            onchange={(e) => appState.updateVideoJob(job.id, { description: (e.target as HTMLTextAreaElement).value })}
            placeholder="Description…"
            rows="3"
            class="w-full text-xs text-content bg-surface rounded p-2 border border-line focus:border-accent focus:outline-none focus:ring-1 focus:ring-accent resize-y"
          ></textarea>
        </div>
      {:else if job.description}
        <div class="mb-2">
          <p class="text-[10px] text-muted font-semibold uppercase tracking-wide mb-1">Description</p>
          <p class="text-xs text-content bg-surface rounded p-2 border border-line">{job.description}</p>
        </div>
      {/if}

      <!-- Title (ausgeklappt) -->
      {#if job.status === 'donePending'}
        <div class="mb-2">
          <p class="text-[10px] text-muted font-semibold uppercase tracking-wide mb-1">Title — editable</p>
          <input
            value={job.title ?? ''}
            onchange={(e) => appState.updateVideoJob(job.id, { title: (e.target as HTMLInputElement).value })}
            placeholder="Title…"
            class="w-full text-xs text-content bg-surface rounded p-2 border border-line font-medium focus:border-accent focus:outline-none focus:ring-1 focus:ring-accent"
          />
        </div>
      {:else if job.title}
        <div class="mb-2">
          <p class="text-[10px] text-muted font-semibold uppercase tracking-wide mb-1">Title</p>
          <p class="text-xs text-content bg-surface rounded p-2 border border-line font-medium">{job.title}</p>
        </div>
      {/if}

      <!-- Genres (ausgeklappt) -->
      {#if job.status === 'donePending'}
        <div class="mb-2">
          <p class="text-[10px] text-muted font-semibold uppercase tracking-wide mb-1">Genres (max 3) — editable</p>
          <EditableChips
            items={job.genres ?? []}
            variant="accent"
            max={3}
            onchange={(g) => appState.updateVideoJob(job.id, { genres: g })}
          />
        </div>
      {:else if job.genres && job.genres.length > 0}
        <div class="mb-2">
          <p class="text-[10px] text-muted font-semibold uppercase tracking-wide mb-1">Genres</p>
          <div class="flex flex-wrap gap-1">
            {#each job.genres as genre}
              <span class="px-2 py-0.5 bg-accent/10 text-accent border border-accent/20 rounded-full text-[10px] font-medium">{genre}</span>
            {/each}
          </div>
        </div>
      {/if}

      <!-- Keywords (Tags) -->
      {#if job.status === 'donePending'}
        <div class="mb-2">
          <p class="text-[10px] text-muted font-semibold uppercase tracking-wide mb-1">Keywords ({job.tags.length}) — editable</p>
          <EditableChips
            items={job.tags}
            variant="neutral"
            onchange={(k) => appState.updateVideoJob(job.id, { tags: k })}
          />
        </div>
      {:else if job.tags.length > 0}
        <div class="mb-2">
          <p class="text-[10px] text-muted font-semibold uppercase tracking-wide mb-1">Keywords ({job.tags.length})</p>
          <div class="flex flex-wrap gap-1">
            {#each job.tags as tag}
              <button onclick={() => appState.copyToClipboard(tag)} title="Copy"
                class="px-2 py-0.5 bg-surface2 text-muted border border-line rounded-full text-[10px] hover:text-content hover:border-accent/40 cursor-pointer">{tag}</button>
            {/each}
          </div>
        </div>
      {:else if job.status === 'done'}
        <p class="text-xs text-muted italic mb-2">No keywords generated.</p>
      {/if}

      <!-- Actions (ausgeklappt) -->
      <div class="flex gap-2 mt-1">
        {#if reasoningPath}
          <button onclick={openReasoning} class="btn btn-warning px-2 py-1 text-[10px] rounded-md">
            🧠 Show Reasoning
          </button>
        {/if}
        {#if job.status === 'done' || job.status === 'error' || job.status === 'donePending'}
          <button onclick={handleRequeue} class="btn btn-secondary px-2 py-1 text-[10px] rounded-md">
            🔄 Re-Queue
          </button>
        {/if}
        {#if job.status !== 'processing' && job.status !== 'extracting'}
          <button onclick={handleRemove} class="btn btn-danger px-2 py-1 text-[10px] rounded-md">
            🗑️ Remove
          </button>
        {/if}
      </div>
    </div>
  {/if}
</div>
