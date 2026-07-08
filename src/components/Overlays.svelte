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

  function kindClasses(kind: string): string {
    switch (kind) {
      case 'success': return 'border-success/40 bg-success/10 text-success';
      case 'error': return 'border-danger/40 bg-danger/10 text-danger';
      default: return 'border-accent/40 bg-accent/10 text-accent';
    }
  }
</script>

<!-- Toasts (unten rechts) -->
<div class="fixed bottom-4 right-4 z-50 flex flex-col gap-2 pointer-events-none">
  {#each appState.toasts as toast (toast.id)}
    <button
      onclick={() => appState.removeToast(toast.id)}
      class="pointer-events-auto max-w-xs text-left px-3 py-2 rounded-lg border shadow-card text-xs backdrop-blur-sm {kindClasses(toast.kind)}"
    >
      {toast.message}
    </button>
  {/each}
</div>

<!-- Confirm-Dialog -->
{#if appState.confirmState}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-sm">
    <div class="card w-80 max-w-[90vw] p-4">
      <p class="text-sm text-content mb-4">{appState.confirmState.message}</p>
      <div class="flex justify-end gap-2">
        <button onclick={() => appState.resolveConfirm(false)} class="btn btn-secondary text-sm">Cancel</button>
        <button onclick={() => appState.resolveConfirm(true)} class="btn btn-danger text-sm">Confirm</button>
      </div>
    </div>
  </div>
{/if}
