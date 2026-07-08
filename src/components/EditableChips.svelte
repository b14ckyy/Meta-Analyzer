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
  interface Props {
    items: string[];
    onchange: (items: string[]) => void;
    variant?: 'accent' | 'neutral';
    max?: number;
    placeholder?: string;
  }

  let { items, onchange, variant = 'neutral', max, placeholder = 'New…' }: Props = $props();

  let adding = $state(false);
  let draft = $state('');

  const chipBase = 'px-2 py-0.5 rounded-full text-[10px] border transition-colors flex items-center gap-1';
  let chipColor = $derived(
    variant === 'accent'
      ? 'bg-accent/10 text-accent border-accent/20'
      : 'bg-surface2 text-muted border-line',
  );
  let canAdd = $derived(!max || items.length < max);

  function remove(i: number) {
    onchange(items.filter((_, idx) => idx !== i));
  }
  function startAdd() { draft = ''; adding = true; }
  function commit() {
    const v = draft.trim();
    if (v && !items.includes(v) && canAdd) {
      onchange([...items, v]);
    }
    draft = '';
    adding = false;
  }
  function cancel() { draft = ''; adding = false; }

  // Fokussiert das Eingabefeld, sobald es erscheint (ohne a11y-autofocus-Warnung).
  function focusEl(node: HTMLInputElement) {
    node.focus();
  }
</script>

<div class="flex flex-wrap gap-1 items-center">
  {#each items as item, i (item + i)}
    <button
      onclick={() => remove(i)}
      title="Remove"
      class="group {chipBase} {chipColor} cursor-pointer hover:!bg-danger/15 hover:!text-danger hover:!border-danger/40"
    >
      <span>{item}</span>
      <span class="opacity-40 group-hover:opacity-100 font-bold leading-none">×</span>
    </button>
  {/each}

  {#if adding}
    <input
      use:focusEl
      value={draft}
      oninput={(e) => (draft = (e.target as HTMLInputElement).value)}
      onkeydown={(e) => { if (e.key === 'Enter') commit(); else if (e.key === 'Escape') cancel(); }}
      onblur={commit}
      {placeholder}
      class="px-2 py-0.5 rounded-full text-[10px] bg-surface border border-accent/50 text-content focus:outline-none focus:ring-1 focus:ring-accent w-24"
    />
  {:else if canAdd}
    <button
      onclick={startAdd}
      title="Add"
      class="{chipBase} bg-surface2 text-muted border-line cursor-pointer hover:text-accent hover:border-accent/40 font-bold"
    >
      +
    </button>
  {/if}
</div>
