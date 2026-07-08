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
  import { onMount } from 'svelte';
  import { appState } from '../lib/store.svelte';
  import { commands } from '../lib/tauri';
  import DualRangeSlider from './DualRangeSlider.svelte';
  import type { VocabularyMode } from '../lib/types';

  let isSaving = $state(false);
  let saveStatus = $state('');
  let newProfileName = $state('');

  // Populated from the backend: hardcoded General/Custom + external video_* rule
  // files (and any dropped-in packs). The fallback covers a failed command call.
  let videoContentTypes = $state<string[]>(['General', 'Custom']);

  onMount(async () => {
    try {
      const types = await commands.listVideoContentTypes();
      if (types.length) videoContentTypes = types.map((t) => t.label);
    } catch (e) {
      console.error('Failed to list video content types:', e);
    }
  });

  const vocabModes: { value: VocabularyMode; label: string }[] = [
    { value: 'Strict', label: 'Strict' },
    { value: 'Recommended', label: 'Recommended' },
    { value: 'Optional', label: 'Optional' },
  ];

  // ── Profile list refresh ──

  async function refreshProfileList() {
    try {
      const names = await commands.listVideoProfiles();
      appState.setVideoProfileNames(names);
    } catch (e) {
      console.error('Failed to list video profiles:', e);
    }
  }

  // ── Profile selection ──

  async function handleProfileSelect(e: Event) {
    const name = (e.target as HTMLSelectElement).value;
    if (!name) return;
    try {
      const profile = await commands.loadVideoProfile(name);
      appState.setActiveVideoProfile(profile);
    } catch (err) {
      saveStatus = `Error loading profile: ${err}`;
    }
  }

  // ── Save As (new profile) ──

  let showSaveAs = $state(false);

  async function handleSaveAs() {
    if (!newProfileName.trim()) return;
    isSaving = true;
    saveStatus = '';
    try {
      const profile = { ...appState.activeVideoProfile, name: newProfileName.trim() };
      await commands.saveVideoProfile(profile);
      appState.setActiveVideoProfile(profile);
      newProfileName = '';
      showSaveAs = false;
      saveStatus = `Video profile "${profile.name}" saved.`;
      await refreshProfileList();
      setTimeout(() => (saveStatus = ''), 2000);
    } catch (err) {
      saveStatus = `Error: ${err}`;
    } finally {
      isSaving = false;
    }
  }

  // ── Save current profile ──

  async function handleSave() {
    isSaving = true;
    saveStatus = '';
    try {
      await commands.saveVideoProfile(appState.activeVideoProfile);
      saveStatus = `Video profile "${appState.activeVideoProfile.name}" saved.`;
      await refreshProfileList();
      setTimeout(() => (saveStatus = ''), 2000);
    } catch (err) {
      saveStatus = `Error: ${err}`;
    } finally {
      isSaving = false;
    }
  }

  // ── Delete profile ──

  async function handleDelete() {
    if (!(await appState.confirm(`Delete video profile "${appState.activeVideoProfile.name}"?`))) return;
    try {
      await commands.deleteVideoProfile(appState.activeVideoProfile.name);
      await refreshProfileList();
      const names = appState.videoProfileNames;
      if (names.length > 0) {
        const next = await commands.loadVideoProfile(names[0]);
        appState.setActiveVideoProfile(next);
      }
      saveStatus = 'Video profile deleted.';
      setTimeout(() => (saveStatus = ''), 2000);
    } catch (err) {
      saveStatus = `Error: ${err}`;
    }
  }

  function setVocabMode(mode: VocabularyMode) {
    appState.updateActiveVideoProfile({ vocabularyMode: mode });
  }
</script>

<div class="space-y-3">
  <!-- Profile selector -->
  <div>
    <label class="block text-xs font-medium text-muted mb-1">Video Profile</label>
    <div class="flex gap-1">
      <select
        value={appState.activeVideoProfile.name}
        onchange={handleProfileSelect}
        class="flex-1 px-2 py-1 bg-surface2 border border-line text-content rounded-lg text-xs focus:border-accent focus:outline-none focus:ring-1 focus:ring-accent"
      >
        {#each appState.videoProfileNames as name}
          <option value={name}>{name}</option>
        {/each}
      </select>
      <button
        onclick={handleSave}
        disabled={isSaving}
        class="btn btn-primary px-2 py-1 text-xs"
      >
        Save
      </button>
      <button
        onclick={() => (showSaveAs = !showSaveAs)}
        class="btn btn-secondary px-2 py-1 text-xs"
      >
        Save As
      </button>
      <button
        onclick={handleDelete}
        disabled={appState.videoProfileNames.length <= 1}
        class="btn btn-danger px-2 py-1 text-xs"
      >
        Delete
      </button>
    </div>

    {#if showSaveAs}
      <div class="flex gap-1 mt-1">
        <input
          type="text"
          placeholder="New profile name..."
          bind:value={newProfileName}
          class="flex-1 px-2 py-1 bg-surface2 border border-line text-content rounded-lg text-xs focus:border-accent focus:outline-none focus:ring-1 focus:ring-accent placeholder:text-muted"
        />
        <button
          onclick={handleSaveAs}
          disabled={isSaving || !newProfileName.trim()}
          class="btn btn-primary px-2 py-1 text-xs"
        >
          Create
        </button>
      </div>
    {/if}
  </div>

  <!-- Custom Prompt (Video-spezifisch! Wird VOR dem generierten Prompt eingefügt) -->
  <div>
    <label for="video-custom-prompt" class="block text-xs font-medium text-muted mb-1">
      Custom Instructions
      <span class="text-muted font-normal"> (inserted before the generated prompt)</span>
    </label>
    <textarea
      id="video-custom-prompt"
      value={appState.activeVideoProfile.customPrompt}
      onchange={(e) => appState.updateActiveVideoProfile({ customPrompt: (e.target as HTMLTextAreaElement).value })}
      rows="3"
      class="w-full px-2 py-1 bg-surface2 border border-line text-content rounded-lg text-xs font-mono resize-y focus:border-accent focus:outline-none focus:ring-1 focus:ring-accent placeholder:text-muted"
      placeholder="Describe your content in your own words. This will be added at the start of the prompt."
    ></textarea>
  </div>

  <!-- Tags range -->
  <div>
    <label class="block text-xs font-medium text-muted mb-1">Number of tags</label>
    <DualRangeSlider
      min={1}
      max={30}
      valueMin={appState.activeVideoProfile.minTags}
      valueMax={appState.activeVideoProfile.maxTags}
      onchange={(min, max) => appState.updateActiveVideoProfile({ minTags: min, maxTags: max })}
    />
  </div>

  <!-- Language -->
  <div>
    <label for="video-lang" class="block text-xs font-medium text-muted mb-1">Language</label>
    <input
      id="video-lang"
      type="text"
      value={appState.activeVideoProfile.language}
      onchange={(e) => appState.updateActiveVideoProfile({ language: (e.target as HTMLInputElement).value })}
      class="w-full px-2 py-1 bg-surface2 border border-line text-content rounded-lg text-xs focus:border-accent focus:outline-none focus:ring-1 focus:ring-accent placeholder:text-muted"
      placeholder="German"
    />
  </div>

  <!-- Content Type (Video) -->
  <div>
    <div class="flex items-center justify-between mb-1">
      <label for="video-content-type" class="block text-xs font-medium text-muted">Content Type</label>
      <button
        type="button"
        onclick={() => commands.openContentRulesDir()}
        class="flex items-center gap-1 text-[10px] text-muted hover:text-accent transition-colors"
        title="Open the content-rules folder to add or edit categories"
      >
        <svg class="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
        </svg>
        Rules folder
      </button>
    </div>
    <select
      id="video-content-type"
      value={appState.activeVideoProfile.contentType}
      onchange={(e) => appState.updateActiveVideoProfile({ contentType: (e.target as HTMLSelectElement).value })}
      class="w-full px-2 py-1 bg-surface2 border border-line text-content rounded-lg text-xs focus:border-accent focus:outline-none focus:ring-1 focus:ring-accent"
    >
      {#each videoContentTypes as ct}
        <option value={ct}>{ct}</option>
      {/each}
    </select>
  </div>

  <!-- Custom Vocabulary -->
  <div>
    <label for="video-custom-vocab" class="block text-xs font-medium text-muted mb-1">
      Custom Vocabulary
      <span class="text-muted font-normal"> (comma-separated)</span>
    </label>
    <textarea
      id="video-custom-vocab"
      value={appState.activeVideoProfile.customVocabulary}
      onchange={(e) => appState.updateActiveVideoProfile({ customVocabulary: (e.target as HTMLTextAreaElement).value })}
      rows="3"
      class="w-full px-2 py-1 bg-surface2 border border-line text-content rounded-lg text-xs font-mono resize-y focus:border-accent focus:outline-none focus:ring-1 focus:ring-accent placeholder:text-muted"
      placeholder="tag1, tag2, tag3"
    ></textarea>
  </div>

  <!-- Vocabulary Mode -->
  <div>
    <span class="block text-xs font-medium text-muted mb-1">Vocabulary Mode</span>
    <div class="flex gap-2">
      {#each vocabModes as vm}
        <label class="flex items-center gap-1 text-xs text-content cursor-pointer">
          <input
            type="radio"
            name="video-vocab-mode"
            checked={appState.activeVideoProfile.vocabularyMode === vm.value}
            onchange={() => setVocabMode(vm.value)}
            class="accent-[rgb(var(--c-accent))]"
          />
          {vm.label}
        </label>
      {/each}
    </div>
  </div>

  <!-- Status message -->
  {#if saveStatus}
    <p class="text-xs text-center {saveStatus.startsWith('Error') ? 'text-danger' : 'text-success'}">
      {saveStatus}
    </p>
  {/if}
</div>
