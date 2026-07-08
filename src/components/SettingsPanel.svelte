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
  import PromptBuilder from './PromptBuilder.svelte';
  import VideoPromptBuilder from './VideoPromptBuilder.svelte';

  // Settings-Panel beim Start eingeklappt.
  let showPanel = $state(false);
  let showPromptPreview = $state(false);
  let isFetchingModels = $state(false);
  let saveStatus = $state('');

  // Favoriten zuerst, dann der Rest — für Dropdown-Reihenfolge.
  let sortedModels = $derived([
    ...appState.availableModels.filter((m) => appState.isFavoriteModel(m)),
    ...appState.availableModels.filter((m) => !appState.isFavoriteModel(m)),
  ]);

  // Prompt-Vorschau vom Backend (entspricht exakt dem, was ans LLM geht).
  let promptPreview = $state('');

  $effect(() => {
    const profile = appState.activeProfile;
    if (appState.appMode === 'photo') {
      commands.previewPrompt(profile).then((result) => {
        promptPreview = result;
      }).catch(() => {
        promptPreview = 'Error generating preview.';
      });
    }
  });

  // Video-Prompt-Vorschau
  $effect(() => {
    const vp = appState.activeVideoProfile;
    if (appState.appMode === 'video') {
      commands.previewVideoPrompt(vp).then((result) => {
        promptPreview = result;
      }).catch(() => {
        promptPreview = 'Error generating video preview.';
      });
    }
  });

  async function fetchModels() {
    isFetchingModels = true;
    saveStatus = '';
    const ok = await appState.refreshModels(appState.settings.apiUrl);
    if (!ok) saveStatus = 'Error fetching models (is LM Studio running?)';
    isFetchingModels = false;
  }

  // Speichert AUSSCHLIESSLICH die App-Settings (inkl. videoSettings). Prompt-Profile
  // werden separat über den (Video-)Prompt-Builder gespeichert.
  async function handleSave() {
    try {
      appState.updateSettings({ videoSettings: appState.videoSettings });
      await commands.saveSettings(appState.settings);
      saveStatus = 'Settings saved!';
      setTimeout(() => { saveStatus = ''; }, 2000);
    } catch (error) {
      saveStatus = `Error: ${error}`;
    }
  }

  function togglePanel() {
    showPanel = !showPanel;
  }
</script>

<div class="flex flex-col min-h-0 border-t border-line h-full bg-surface">
  <!-- ── Einklappbare Settings ── -->
  <button
    onclick={togglePanel}
    class="shrink-0 w-full px-4 py-3 bg-surface2 hover:bg-surface text-content font-semibold text-left text-sm border-b border-line"
  >
    {showPanel ? '▼' : '▶'} Settings
  </button>

  {#if showPanel}
    <div class="shrink-0 max-h-[55%] overflow-y-auto p-3 space-y-3 border-b border-line">

      <!-- API URL/Key + Model → nebeneinander -->
      <div class="grid grid-cols-2 gap-2">
        <div class="space-y-2">
          <div>
            <label for="api-url" class="block text-xs font-medium text-muted mb-1">API URL</label>
            <input
              id="api-url"
              type="text"
              value={appState.settings.apiUrl}
              onchange={(e) => appState.updateSettings({ apiUrl: (e.target as HTMLInputElement).value })}
              class="w-full px-2 py-1 bg-surface2 border border-line text-content rounded-lg text-xs focus:border-accent focus:outline-none focus:ring-1 focus:ring-accent placeholder:text-muted"
              placeholder="http://localhost:1234"
            />
          </div>

          <!-- API Key (optional, for cloud / private AI servers) -->
          <div>
            <label for="api-key" class="block text-[10px] font-medium text-muted mb-0.5">
              API Key <span class="text-muted/60">(optional)</span>
            </label>
            <input
              id="api-key"
              type="password"
              value={appState.settings.apiKey}
              onchange={(e) => appState.updateSettings({ apiKey: (e.target as HTMLInputElement).value })}
              class="w-full px-2 py-0.5 bg-surface2 border border-line text-content rounded-lg text-[11px] focus:border-accent focus:outline-none focus:ring-1 focus:ring-accent placeholder:text-muted"
              placeholder="sk-… (empty for local)"
              autocomplete="off"
            />
          </div>
        </div>

        <div>
          <div class="flex justify-between items-center mb-1">
            <span class="text-xs font-medium text-muted flex items-center gap-1.5">
              Model
              <span
                class="w-1.5 h-1.5 rounded-full {appState.modelsOnline ? 'bg-success' : 'bg-muted/50'}"
                title={appState.modelsOnline ? 'LM Studio connected' : 'Waiting for LM Studio…'}
              ></span>
            </span>
            <button
              onclick={fetchModels}
              disabled={isFetchingModels}
              title="Reload models"
              class="text-[10px] bg-accent text-white px-1.5 py-0.5 rounded-lg disabled:opacity-40 hover:brightness-110"
            >
              {isFetchingModels ? '...' : '↻'}
            </button>
          </div>

          <!-- Favoriten (angeheftete Modelle) für Schnellauswahl -->
          {#if appState.favoriteModels.length > 0}
            <div class="flex flex-wrap gap-1 mb-1">
              {#each appState.favoriteModels as fav}
                <button
                  onclick={() => appState.updateSettings({ modelName: fav })}
                  title={fav}
                  class="flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] border transition-colors
                    {appState.settings.modelName === fav
                      ? 'bg-accent/15 text-accent border-accent/40'
                      : 'bg-surface2 text-muted border-line hover:text-content'}"
                >
                  <span class="text-accent">★</span>
                  <span class="truncate max-w-[110px]">{fav}</span>
                </button>
              {/each}
            </div>
          {/if}

          {#if appState.availableModels.length > 0}
            <select
              value={appState.settings.modelName}
              onchange={(e) => appState.updateSettings({ modelName: (e.target as HTMLSelectElement).value })}
              class="w-full px-2 py-1 bg-surface2 border border-line text-content rounded-lg text-xs mb-1 focus:border-accent focus:outline-none focus:ring-1 focus:ring-accent"
            >
              <option value="">-- Select --</option>
              {#each sortedModels as model}
                <option value={model}>{appState.isFavoriteModel(model) ? '★ ' : ''}{model}</option>
              {/each}
            </select>
          {/if}

          <div class="flex gap-1">
            <input
              id="model-name"
              type="text"
              value={appState.settings.modelName}
              onchange={(e) => appState.updateSettings({ modelName: (e.target as HTMLInputElement).value })}
              class="flex-1 min-w-0 px-2 py-1 bg-surface2 border border-line text-content rounded-lg text-xs focus:border-accent focus:outline-none focus:ring-1 focus:ring-accent placeholder:text-muted"
              placeholder="Model name (or ↻)"
            />
            <button
              onclick={() => appState.toggleFavoriteModel(appState.settings.modelName)}
              disabled={!appState.settings.modelName}
              title={appState.isFavoriteModel(appState.settings.modelName) ? 'Remove from favorites' : 'Add to favorites'}
              class="shrink-0 w-7 rounded-lg border border-line bg-surface2 text-sm leading-none disabled:opacity-40 transition-colors
                {appState.isFavoriteModel(appState.settings.modelName) ? 'text-accent' : 'text-muted hover:text-content'}"
            >
              {appState.isFavoriteModel(appState.settings.modelName) ? '★' : '☆'}
            </button>
          </div>
        </div>
      </div>

      <!-- Photo: Workers + Skip/Apply — Video: Frames + Workers + Metadata -->
      {#if appState.appMode === 'video'}
        <div class="grid grid-cols-2 gap-2">
          <div>
            <label for="num-frames" class="block text-xs font-medium text-muted mb-1">
              Snapshots: {appState.videoSettings.numFrames}
            </label>
            <input
              id="num-frames"
              type="range"
              min="3"
              max="50"
              step="1"
              value={appState.videoSettings.numFrames}
              oninput={(e) => appState.updateVideoSettings({ numFrames: parseInt((e.target as HTMLInputElement).value) })}
              class="w-full accent-[rgb(var(--c-accent))]"
            />
            <div class="flex justify-between text-[10px] text-muted px-0.5">
              <span>3</span><span>15</span><span>30</span><span>50</span>
            </div>
          </div>

          <div>
            <label for="video-workers" class="block text-xs font-medium text-muted mb-1">
              Video Workers: {appState.videoSettings.maxConcurrent}
            </label>
            <input
              id="video-workers"
              type="range"
              min="1"
              max="4"
              step="1"
              value={appState.videoSettings.maxConcurrent}
              oninput={(e) => appState.updateVideoSettings({ maxConcurrent: parseInt((e.target as HTMLInputElement).value) })}
              class="w-full accent-[rgb(var(--c-accent))]"
            />
            <div class="flex justify-between text-[10px] text-muted px-0.5">
              <span>1</span><span>2</span><span>3</span><span>4</span>
            </div>
          </div>
        </div>

        <!-- Video Metadata Checkboxen -->
        <div class="border-t border-line pt-2">
          <h3 class="text-xs font-semibold text-content mb-1">Write metadata</h3>
          <div class="grid grid-cols-3 gap-1">
            <label class="flex items-center gap-1 text-xs text-muted cursor-pointer">
              <input
                type="checkbox"
                checked={appState.videoSettings.writeDescription}
                onchange={(e) => appState.updateVideoSettings({ writeDescription: (e.target as HTMLInputElement).checked })}
                class="rounded accent-[rgb(var(--c-accent))]"
              />
              Description
            </label>
            <label class="flex items-center gap-1 text-xs text-muted cursor-pointer">
              <input
                type="checkbox"
                checked={appState.videoSettings.writeGenres}
                onchange={(e) => appState.updateVideoSettings({ writeGenres: (e.target as HTMLInputElement).checked })}
                class="rounded accent-[rgb(var(--c-accent))]"
              />
              Genres
            </label>
            <label class="flex items-center gap-1 text-xs text-muted cursor-pointer">
              <input
                type="checkbox"
                checked={appState.videoSettings.writeTags}
                onchange={(e) => appState.updateVideoSettings({ writeTags: (e.target as HTMLInputElement).checked })}
                class="rounded accent-[rgb(var(--c-accent))]"
              />
              Tags
            </label>
            <label class="flex items-center gap-1 text-xs text-muted cursor-pointer">
              <input
                type="checkbox"
                checked={appState.videoSettings.writeTitle}
                onchange={(e) => appState.updateVideoSettings({ writeTitle: (e.target as HTMLInputElement).checked })}
                class="rounded accent-[rgb(var(--c-accent))]"
              />
              Title
            </label>
          </div>
          <label class="flex items-center gap-1 text-xs text-muted cursor-pointer mt-1">
            <input
              type="checkbox"
              checked={appState.videoSettings.applyAutomatically}
              onchange={(e) => appState.updateVideoSettings({ applyAutomatically: (e.target as HTMLInputElement).checked })}
              class="rounded accent-[rgb(var(--c-accent))]"
            />
            Apply Automatically
          </label>
        </div>
      {:else}
        <div class="grid grid-cols-2 gap-2">
          <div>
            <label for="max-concurrent" class="block text-xs font-medium text-muted mb-1">
              Workers: {appState.settings.maxConcurrent}
            </label>
            <input
              id="max-concurrent"
              type="range"
              min="1"
              max="8"
              step="1"
              value={appState.settings.maxConcurrent}
              oninput={(e) => appState.updateSettings({ maxConcurrent: parseInt((e.target as HTMLInputElement).value) })}
              class="w-full accent-[rgb(var(--c-accent))]"
            />
            <div class="flex justify-between text-[10px] text-muted px-0.5">
              <span>1</span><span>3</span><span>6</span><span>8</span>
            </div>
          </div>

          <div class="flex flex-col justify-center gap-1.5 pt-5">
            <label class="flex items-center gap-2 text-xs font-medium text-muted cursor-pointer">
              <input
                type="checkbox"
                checked={appState.settings.skipTagged}
                onchange={(e) => appState.updateSettings({ skipTagged: (e.target as HTMLInputElement).checked })}
                class="rounded accent-[rgb(var(--c-accent))]"
              />
              Skip tagged
            </label>
            <label class="flex items-center gap-2 text-xs font-medium text-muted cursor-pointer">
              <input
                type="checkbox"
                checked={appState.settings.applyAutomatically}
                onchange={(e) => appState.updateSettings({ applyAutomatically: (e.target as HTMLInputElement).checked })}
                class="rounded accent-[rgb(var(--c-accent))]"
              />
              Apply automatically
            </label>
          </div>
        </div>
      {/if}

      <!-- Save (nur Settings) -->
      <button onclick={handleSave} class="btn btn-primary w-full text-sm">
        Save Settings
      </button>

      {#if saveStatus}
        <p class="text-xs text-center {saveStatus.startsWith('Error') ? 'text-danger' : 'text-success'}">
          {saveStatus}
        </p>
      {/if}
    </div>
  {/if}

  <!-- ── Prompt Builder (immer sichtbar) ── -->
  <div class="flex-1 min-h-0 overflow-y-auto p-3 space-y-3">
    <div>
      <h3 class="text-xs font-semibold text-content mb-2">
        {appState.appMode === 'video' ? 'Video Prompt Builder' : 'Prompt Builder'}
      </h3>
      {#if appState.appMode === 'video'}
        <VideoPromptBuilder />
      {:else}
        <PromptBuilder />
      {/if}
    </div>

    <!-- Prompt Preview (collapsible) -->
    <div class="border-t border-line pt-2">
      <button
        onclick={() => (showPromptPreview = !showPromptPreview)}
        class="flex items-center gap-1 text-xs font-semibold text-muted hover:text-content w-full text-left"
      >
        {showPromptPreview ? '▼' : '▶'} Prompt Preview
      </button>

      {#if showPromptPreview}
        <pre class="mt-1 p-2 bg-base border border-line text-muted rounded-lg text-[10px] font-mono leading-relaxed overflow-x-auto whitespace-pre-wrap max-h-60 overflow-y-auto">{promptPreview}</pre>
      {/if}
    </div>
  </div>
</div>
