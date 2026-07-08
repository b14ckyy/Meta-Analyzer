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

  type Mode = 'hidden' | 'small' | 'expanded';

  interface Props {
    mode: Mode;
    onModeChange: (m: Mode) => void;
  }

  let { mode, onModeChange }: Props = $props();

  let container = $state<HTMLDivElement | null>(null);
  let selectedTab: string | null = $state(null);
  // Autoscroll folgt dem Stream, solange man (nahe) unten ist. Scrollt man hoch,
  // geht es aus; scrollt man wieder ganz runter, geht es automatisch wieder an.
  let stick = $state(true);

  // Wähle die aktive Buffer-Liste basierend auf Modus
  let bufferList = $derived(
    appState.appMode === 'video' ? appState.videoWorkerBuffers : appState.workerBuffers
  );

  // Auto-select first active worker
  $effect(() => {
    if (bufferList.length > 0 && (!selectedTab || !bufferList.find((w) => w.jobId === selectedTab))) {
      selectedTab = bufferList[0].jobId;
    }
  });

  // Beim Tab-Wechsel wieder dem Ende folgen (neuester Stand sichtbar).
  $effect(() => {
    void selectedTab;
    stick = true;
  });

  // Bindet den Scroll-Container und misst beim Scrollen die Position.
  function bindContainer(node: HTMLDivElement) {
    container = node;
    return { destroy() { if (container === node) container = null; } };
  }

  function onScroll() {
    const el = container;
    if (!el) return;
    const distanceFromBottom = el.scrollHeight - el.scrollTop - el.clientHeight;
    stick = distanceFromBottom < 40; // nahe genug am Ende → Autoscroll an
  }

  // Ans Ende scrollen, wenn neuer Content kommt — aber nur solange gestickt wird.
  $effect(() => {
    const w = activeWorker;
    // Content-Wachstum als Abhängigkeit tracken:
    void ((w?.reasoning.length ?? 0) + (w?.content.length ?? 0));
    void selectedTab;
    if (stick && mode !== 'hidden' && container) {
      const el = container;
      // Nach dem DOM-Update (neue Höhe) ans Ende scrollen.
      requestAnimationFrame(() => { if (stick && el) el.scrollTop = el.scrollHeight; });
    }
  });

  let activeWorker = $derived(
    selectedTab ? bufferList.find((w) => w.jobId === selectedTab) : null,
  );

  function formatTokens(tps: number): string {
    if (tps < 1) return '<1 tok/s';
    if (tps > 1000) return `${(tps / 1000).toFixed(1)}k tok/s`;
    return `${Math.round(tps)} tok/s`;
  }

  function nextMode() {
    if (mode === 'expanded') onModeChange('small');
    else if (mode === 'small') onModeChange('hidden');
    else onModeChange('expanded');
  }

  let nextLabel = $derived(
    mode === 'expanded' ? 'Shrink ▽' : mode === 'small' ? 'Hide ✕' : 'Expand △',
  );

</script>

<div class="flex flex-col border-t border-line bg-surface text-content h-full min-h-0">
  <!-- Header -->
  <div class="px-4 py-2 bg-surface2 border-b border-line flex items-center justify-between shrink-0">
    <div class="flex items-center gap-2 min-w-0">
      {#if bufferList.length > 0}
        <div class="w-2 h-2 rounded-full bg-accent animate-pulse shrink-0"></div>
        <span class="text-sm font-medium truncate">
          {bufferList.length} active worker{bufferList.length !== 1 ? 's' : ''}
        </span>
      {:else}
        <div class="w-2 h-2 rounded-full bg-muted shrink-0"></div>
        <span class="text-sm text-muted">Thinking panel (idle)</span>
      {/if}
    </div>

    <div class="flex items-center gap-3 shrink-0">
      {#if appState.appMode === 'video'}
        {#if appState.videoAvgSecondsPerJob > 0}
          <span class="text-xs text-muted">
            ~{appState.videoAvgSecondsPerJob.toFixed(1)}s/job
          </span>
        {/if}
        {#if appState.videoElapsedSeconds > 0}
          <span class="text-xs text-muted">{appState.videoElapsedSeconds}s</span>
        {/if}
      {:else}
        {#if appState.avgSecondsPerJob > 0}
          <span class="text-xs text-muted">
            ~{appState.avgSecondsPerJob.toFixed(1)}s/job
          </span>
        {/if}
        {#if appState.elapsedSeconds > 0}
          <span class="text-xs text-muted">{appState.elapsedSeconds}s</span>
        {/if}
      {/if}
      <button
        onclick={nextMode}
        class="text-xs px-2 py-1 rounded-lg bg-surface2 hover:border-accent text-content border border-line"
        title="Toggle thinking panel size"
      >
        {nextLabel}
      </button>
    </div>
  </div>

  <!-- Body -->
  {#if mode !== 'hidden'}
    {#if bufferList.length > 0}
      <!-- Tab bar for workers -->
      <div class="flex gap-1 px-4 pt-2 pb-1 overflow-x-auto shrink-0 bg-surface2 border-b border-line">
        {#each bufferList as worker}
          {@const streaming = (worker.reasoning?.length ?? 0) > 0 || (worker.content?.length ?? 0) > 0}
          <button
            onclick={() => { selectedTab = worker.jobId; }}
            class="text-xs px-2 py-1 rounded-lg whitespace-nowrap flex items-center gap-1 border transition-colors
              {selectedTab === worker.jobId
                ? 'bg-accent/15 text-accent'
                : 'bg-surface text-muted hover:text-content'}
              {streaming ? 'border-accent/60 shadow-glow' : 'border-transparent'}"
          >
            {#if streaming}
              <span class="w-1.5 h-1.5 rounded-full bg-accent animate-pulse shrink-0"></span>
            {/if}
            {worker.fileName.length > 16 ? worker.fileName.slice(0, 14) + '…' : worker.fileName}
            {#if worker.tokensPerSecond > 0}
              <span class="ml-1 text-muted">({formatTokens(worker.tokensPerSecond)})</span>
            {/if}
          </button>
        {/each}
      </div>

      <!-- Active worker content -->
      {#if activeWorker}
        <div
          use:bindContainer
          onscroll={onScroll}
          class="flex-1 min-h-0 overflow-y-auto px-4 py-2 font-mono text-xs leading-relaxed bg-base text-muted"
        >
          {#if activeWorker.reasoning}
            <div class="text-muted whitespace-pre-wrap">{activeWorker.reasoning}</div>
          {/if}

          {#if activeWorker.content}
            <div class="mt-3 pt-3 border-t border-line bg-surface text-content rounded-lg p-2">
              <div class="text-xs text-success font-semibold mb-1">Answer:</div>
              <div class="text-content whitespace-pre-wrap break-all">{activeWorker.content}</div>
            </div>
          {/if}

          {#if activeWorker.usage}
            <div class="mt-3 pt-3 border-t border-line text-xs text-muted">
              Tokens: {activeWorker.usage.prompt} prompt / {activeWorker.usage.completion} completion
              {#if activeWorker.tokensPerSecond > 0}
                · {formatTokens(activeWorker.tokensPerSecond)}
              {/if}
            </div>
          {/if}

          {#if !activeWorker.reasoning && !activeWorker.content}
            <div class="text-muted italic">Waiting for response…</div>
          {/if}
        </div>
      {/if}
    {:else}
      <!-- Empty state -->
      <div class="flex-1 min-h-0 overflow-y-auto px-4 py-2 font-mono text-xs leading-relaxed bg-base">
        <div class="text-muted italic">
          No active jobs. Live reasoning and model output will appear here while processing.
        </div>
    </div>
    {/if}
  {/if}
</div>
