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
  import type { AppMode } from '../lib/types';

  function setMode(mode: AppMode) {
    appState.setAppMode(mode);
  }
</script>

<div class="flex items-center gap-2 p-3 border-b border-line bg-surface2">
  <!-- Brand -->
  <div class="flex items-center gap-2 pr-1">
    <div class="w-6 h-6 rounded-md flex items-center justify-center text-white shadow-glow"
      style="background-image: linear-gradient(135deg, rgb(var(--c-accent)), rgb(var(--c-accent2)));">
      <svg viewBox="0 0 24 24" fill="currentColor" class="w-3.5 h-3.5"><path d="M12 2l2.4 6.9L21 11l-6.6 2.1L12 20l-2.4-6.9L3 11l6.6-2.1z" /></svg>
    </div>
  </div>

  <!-- Segmented mode control -->
  <div class="flex-1 flex gap-1 p-1 rounded-lg bg-base border border-line">
    <button
      onclick={() => setMode('photo')}
      class="flex-1 px-3 py-1.5 text-sm font-medium rounded-md transition-colors
        {appState.appMode === 'photo'
          ? 'text-white shadow-glow'
          : 'text-muted hover:text-content'}"
      style={appState.appMode === 'photo'
        ? 'background-image: linear-gradient(135deg, rgb(var(--c-accent)), rgb(var(--c-accent2)));'
        : ''}
    >
      📷 Photo
    </button>
    <button
      onclick={() => setMode('video')}
      class="flex-1 px-3 py-1.5 text-sm font-medium rounded-md transition-colors
        {appState.appMode === 'video'
          ? 'text-white shadow-glow'
          : 'text-muted hover:text-content'}"
      style={appState.appMode === 'video'
        ? 'background-image: linear-gradient(135deg, rgb(var(--c-accent)), rgb(var(--c-accent2)));'
        : ''}
    >
      🎬 Video
    </button>
  </div>

  <!-- Theme toggle -->
  <button
    onclick={() => appState.toggleTheme()}
    title={appState.theme === 'dark' ? 'Switch to light theme' : 'Switch to dark theme'}
    aria-label="Toggle theme"
    class="w-9 h-9 shrink-0 rounded-lg bg-surface border border-line text-muted
      hover:text-content hover:border-accent/50 flex items-center justify-center transition-colors"
  >
    {#if appState.theme === 'dark'}
      <!-- Sun -->
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" class="w-4 h-4">
        <circle cx="12" cy="12" r="4" />
        <path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4" />
      </svg>
    {:else}
      <!-- Moon -->
      <svg viewBox="0 0 24 24" fill="currentColor" class="w-4 h-4">
        <path d="M21 12.8A9 9 0 1111.2 3a7 7 0 009.8 9.8z" />
      </svg>
    {/if}
  </button>
</div>
