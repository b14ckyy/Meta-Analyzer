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

  // Populated from the backend: hardcoded General/Custom + external image_* rule
  // files (and any dropped-in packs). The fallback covers a failed command call.
  let contentTypes = $state<string[]>(['General', 'Custom']);

  onMount(async () => {
    try {
      const types = await commands.listContentTypes();
      if (types.length) contentTypes = types.map((t) => t.label);
    } catch (e) {
      console.error('Failed to list content types:', e);
    }
  });

  // ── Profile list refresh ──

  async function refreshProfileList() {
    try {
      const names = await commands.listProfiles();
      appState.setProfileNames(names);
    } catch (e) {
      console.error('Failed to list profiles:', e);
    }
  }

  // ── Profile selection ──

  async function handleProfileSelect(e: Event) {
    const name = (e.target as HTMLSelectElement).value;
    if (!name) return;
    try {
      const profile = await commands.loadProfile(name);
      appState.setActiveProfile(profile);
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
      const profile = { ...appState.activeProfile, name: newProfileName.trim() };
      await commands.saveProfile(profile);
      appState.setActiveProfile(profile);
      newProfileName = '';
      showSaveAs = false;
      saveStatus = `Profile "${profile.name}" saved.`;
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
      await commands.saveProfile(appState.activeProfile);
      saveStatus = `Profile "${appState.activeProfile.name}" saved.`;
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
    if (!(await appState.confirm(`Delete profile "${appState.activeProfile.name}"?`))) return;
    try {
      await commands.deleteProfile(appState.activeProfile.name);
      await refreshProfileList();
      // Switch to first available or create new default
      const names = appState.profileNames;
      if (names.length > 0) {
        const next = await commands.loadProfile(names[0]);
        appState.setActiveProfile(next);
      }
      saveStatus = 'Profile deleted.';
      setTimeout(() => (saveStatus = ''), 2000);
    } catch (err) {
      saveStatus = `Error: ${err}`;
    }
  }

  // ── Vocabulary mode helper ──

  const vocabModes: { value: VocabularyMode; label: string }[] = [
    { value: 'Strict', label: 'Strict' },
    { value: 'Recommended', label: 'Recommended' },
    { value: 'Optional', label: 'Optional' },
  ];

  function setVocabMode(mode: VocabularyMode) {
    appState.updateActiveProfile({ vocabularyMode: mode });
  }
</script>

<div class="space-y-3">
  <!-- Profile selector -->
  <div>
    <label class="block text-xs font-medium text-muted mb-1">Profile</label>
    <div class="flex gap-1">
      <select
        value={appState.activeProfile.name}
        onchange={handleProfileSelect}
        class="flex-1 px-2 py-1 bg-surface2 border border-line text-content rounded-lg text-xs focus:border-accent focus:outline-none focus:ring-1 focus:ring-accent"
      >
        {#each appState.profileNames as name}
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
        disabled={appState.profileNames.length <= 1}
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

  <!-- Tags range -->
  <div>
    <label class="block text-xs font-medium text-muted mb-1">Number of tags</label>
    <DualRangeSlider
      min={1}
      max={30}
      valueMin={appState.activeProfile.minTags}
      valueMax={appState.activeProfile.maxTags}
      onchange={(min, max) => appState.updateActiveProfile({ minTags: min, maxTags: max })}
    />
  </div>

  <!-- Language -->
  <div>
    <label for="lang" class="block text-xs font-medium text-muted mb-1">Language</label>
    <input
      id="lang"
      type="text"
      value={appState.activeProfile.language}
      onchange={(e) => appState.updateActiveProfile({ language: (e.target as HTMLInputElement).value })}
      class="w-full px-2 py-1 bg-surface2 border border-line text-content rounded-lg text-xs focus:border-accent focus:outline-none focus:ring-1 focus:ring-accent placeholder:text-muted"
      placeholder="English"
    />
  </div>

  <!-- Content Type -->
  <div>
    <div class="flex items-center justify-between mb-1">
      <label for="content-type" class="block text-xs font-medium text-muted">Content Type</label>
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
      id="content-type"
      value={appState.activeProfile.contentType}
      onchange={(e) => appState.updateActiveProfile({ contentType: (e.target as HTMLSelectElement).value })}
      class="w-full px-2 py-1 bg-surface2 border border-line text-content rounded-lg text-xs focus:border-accent focus:outline-none focus:ring-1 focus:ring-accent"
    >
      {#each contentTypes as ct}
        <option value={ct}>{ct}</option>
      {/each}
    </select>
  </div>

  <!-- Custom Instructions (inserted near the start of the prompt) -->
  <div>
    <label for="custom-prompt" class="block text-xs font-medium text-muted mb-1">
      Custom Instructions
      <span class="text-muted font-normal"> (inserted before the generated prompt)</span>
    </label>
    <textarea
      id="custom-prompt"
      value={appState.activeProfile.customPrompt}
      onchange={(e) => appState.updateActiveProfile({ customPrompt: (e.target as HTMLTextAreaElement).value })}
      rows="3"
      class="w-full px-2 py-1 bg-surface2 border border-line text-content rounded-lg text-xs font-mono resize-y focus:border-accent focus:outline-none focus:ring-1 focus:ring-accent placeholder:text-muted"
      placeholder="Describe your content in your own words. This will be added at the start of the prompt."
    ></textarea>
  </div>

  <!-- Custom Vocabulary (always visible) -->
  <div>
    <label for="custom-vocab" class="block text-xs font-medium text-muted mb-1">
      Custom Vocabulary
      <span class="text-muted font-normal"> (comma-separated, added on top of content type vocabulary)</span>
    </label>
    <textarea
      id="custom-vocab"
      value={appState.activeProfile.customVocabulary}
      onchange={(e) => appState.updateActiveProfile({ customVocabulary: (e.target as HTMLTextAreaElement).value })}
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
            name="vocab-mode"
            checked={appState.activeProfile.vocabularyMode === vm.value}
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
