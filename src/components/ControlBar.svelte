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

  // ── Photo Mode ──
  async function handleStart() {
    try {
      if (appState.settings.skipTagged) {
        appState.skipTaggedJobs();
      }
      const pendingJobs = appState.jobs.filter((j) => j.status === 'pending');
      if (pendingJobs.length === 0) return;
      appState.setProcessing(true);
      appState.setPaused(false);
      await commands.startProcessing(appState.jobs, appState.settings, appState.activeProfile);
    } catch (error) {
      console.error('Failed to start processing:', error);
      appState.setProcessing(false);
      appState.addToast(`Error: ${error}`, 'error');
    }
  }

  async function handlePause() {
    try {
      await commands.pauseProcessing();
      appState.setPaused(true);
    } catch (error) {
      console.error('Failed to pause:', error);
      appState.addToast(`Error: ${error}`, 'error');
    }
  }

  async function handleResume() {
    try {
      await commands.resumeProcessing();
      appState.setPaused(false);
    } catch (error) {
      console.error('Failed to resume:', error);
      appState.addToast(`Error: ${error}`, 'error');
    }
  }

  async function handleStop() {
    try {
      await commands.stopProcessing();
      appState.setProcessing(false);
      appState.setPaused(false);
    } catch (error) {
      console.error('Failed to stop:', error);
      appState.addToast(`Error: ${error}`, 'error');
    }
  }

  // ── Video Mode ──
  async function handleVideoStart() {
    try {
      const pendingJobs = appState.videoJobs.filter((j) => j.status === 'pending');
      if (pendingJobs.length === 0) return;
      appState.setVideoProcessing(true);
      appState.setVideoPaused(false);
      await commands.startVideoProcessing(
        appState.videoJobs,
        appState.settings,
        appState.activeVideoProfile,
        appState.videoSettings,
      );
    } catch (error) {
      console.error('Failed to start video processing:', error);
      appState.setVideoProcessing(false);
      appState.addToast(`Error: ${error}`, 'error');
    }
  }

  async function handleVideoPause() {
    try {
      await commands.pauseVideoProcessing();
      appState.setVideoPaused(true);
    } catch (error) {
      console.error('Failed to pause video:', error);
      appState.addToast(`Error: ${error}`, 'error');
    }
  }

  async function handleVideoResume() {
    try {
      await commands.resumeVideoProcessing();
      appState.setVideoPaused(false);
    } catch (error) {
      console.error('Failed to resume video:', error);
      appState.addToast(`Error: ${error}`, 'error');
    }
  }

  async function handleVideoStop() {
    try {
      await commands.stopVideoProcessing();
      appState.setVideoProcessing(false);
      appState.setVideoPaused(false);
    } catch (error) {
      console.error('Failed to stop video processing:', error);
      appState.addToast(`Error: ${error}`, 'error');
    }
  }

  // ── Derived state ──
  let isVideo = $derived(appState.appMode === 'video');
  let processing = $derived(isVideo ? appState.isVideoProcessing : appState.isProcessing);
  let paused = $derived(isVideo ? appState.isVideoPaused : appState.isPaused);

  let startDisabled = $derived(
    isVideo
      ? appState.isVideoProcessing || appState.videoJobs.filter((j) => j.status === 'pending').length === 0
      : appState.isProcessing || appState.jobs.length === 0,
  );
  let controlDisabled = $derived(!processing);

  function onStart() { isVideo ? handleVideoStart() : handleStart(); }
  function onPause() { isVideo ? handleVideoPause() : handlePause(); }
  function onResume() { isVideo ? handleVideoResume() : handleResume(); }
  function onStop() { isVideo ? handleVideoStop() : handleStop(); }
</script>

{#snippet iconPlay()}
  <svg viewBox="0 0 24 24" fill="currentColor" class="w-4 h-4"><path d="M8 5v14l11-7z" /></svg>
{/snippet}
{#snippet iconPause()}
  <svg viewBox="0 0 24 24" fill="currentColor" class="w-4 h-4"><path d="M6 5h3.5v14H6zM14.5 5H18v14h-3.5z" /></svg>
{/snippet}
{#snippet iconStop()}
  <svg viewBox="0 0 24 24" fill="currentColor" class="w-4 h-4"><rect x="6" y="6" width="12" height="12" rx="1.5" /></svg>
{/snippet}

<div class="px-4 py-3 border-b border-line bg-surface flex gap-2">
  <button onclick={onStart} disabled={startDisabled} class="btn btn-primary">
    {@render iconPlay()}
    Start
  </button>

  {#if paused}
    <button onclick={onResume} disabled={controlDisabled} class="btn btn-primary">
      {@render iconPlay()}
      Resume
    </button>
  {:else}
    <button onclick={onPause} disabled={controlDisabled} class="btn btn-warning">
      {@render iconPause()}
      Pause
    </button>
  {/if}

  <button onclick={onStop} disabled={controlDisabled} class="btn btn-danger">
    {@render iconStop()}
    Stop
  </button>
</div>
