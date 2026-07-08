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
    min: number;
    max: number;
    valueMin: number;
    valueMax: number;
    onchange: (min: number, max: number) => void;
  }

  let { min, max, valueMin, valueMax, onchange }: Props = $props();

  const total = $derived(max - min);

  // Re-clamp whenever values change (e.g. programmatic update)
  // DEKLARIERT VOR Verwendung in leftPct/rightPct
  let safeMin = $derived(Math.min(valueMin, valueMax - 1));
  let safeMax = $derived(Math.max(valueMax, valueMin + 1));

  // Position in percent for the track fill
  const leftPct = $derived(((safeMin - min) / total) * 100);
  const rightPct = $derived(((safeMax - min) / total) * 100);

  // ── Synthetic range thumbs via two overlapping <input type="range"> ──
  // Solved: we use two inputs stacked on top of each other.
  // The lower (min) thumb is left, the upper (max) thumb is right.
  // CSS ensures they share the same track.

  function handleMinInput(e: Event) {
    const val = parseInt((e.target as HTMLInputElement).value);
    const clamped = Math.min(val, valueMax - 1);
    onchange(clamped, valueMax);
  }

  function handleMaxInput(e: Event) {
    const val = parseInt((e.target as HTMLInputElement).value);
    const clamped = Math.max(val, valueMin + 1);
    onchange(valueMin, clamped);
  }
</script>

<div class="relative h-10 flex flex-col justify-center">
  <!-- Track background -->
  <div class="absolute inset-x-0 top-1/2 -translate-y-1/2 h-1 bg-line rounded mx-1.5"></div>

  <!-- Active track fill -->
  <div
    class="absolute top-1/2 -translate-y-1/2 h-1 bg-accent rounded mx-1.5"
    style="left: {leftPct}%; right: {100 - rightPct}%"
  ></div>

  <!-- Min range -->
  <input
    type="range"
    {min}
    {max}
    value={valueMin}
    oninput={handleMinInput}
    class="
      absolute inset-x-0 top-0 bottom-0 w-full appearance-none bg-transparent pointer-events-none
      [&::-webkit-slider-thumb]:pointer-events-auto
      [&::-webkit-slider-thumb]:appearance-none
      [&::-webkit-slider-thumb]:w-4
      [&::-webkit-slider-thumb]:h-4
      [&::-webkit-slider-thumb]:rounded-full
      [&::-webkit-slider-thumb]:bg-accent
      [&::-webkit-slider-thumb]:border-2
      [&::-webkit-slider-thumb]:border-surface
      [&::-webkit-slider-thumb]:shadow
      [&::-webkit-slider-thumb]:cursor-pointer
      [&::-moz-range-thumb]:pointer-events-auto
      [&::-moz-range-thumb]:w-4
      [&::-moz-range-thumb]:h-4
      [&::-moz-range-thumb]:rounded-full
      [&::-moz-range-thumb]:bg-accent
      [&::-moz-range-thumb]:border-2
      [&::-moz-range-thumb]:border-surface
      [&::-moz-range-thumb]:shadow
      [&::-moz-range-thumb]:cursor-pointer
    "
  />

  <!-- Max range -->
  <input
    type="range"
    {min}
    {max}
    value={valueMax}
    oninput={handleMaxInput}
    class="
      absolute inset-x-0 top-0 bottom-0 w-full appearance-none bg-transparent pointer-events-none
      [&::-webkit-slider-thumb]:pointer-events-auto
      [&::-webkit-slider-thumb]:appearance-none
      [&::-webkit-slider-thumb]:w-4
      [&::-webkit-slider-thumb]:h-4
      [&::-webkit-slider-thumb]:rounded-full
      [&::-webkit-slider-thumb]:bg-accent
      [&::-webkit-slider-thumb]:border-2
      [&::-webkit-slider-thumb]:border-surface
      [&::-webkit-slider-thumb]:shadow
      [&::-webkit-slider-thumb]:cursor-pointer
      [&::-moz-range-thumb]:pointer-events-auto
      [&::-moz-range-thumb]:w-4
      [&::-moz-range-thumb]:h-4
      [&::-moz-range-thumb]:rounded-full
      [&::-moz-range-thumb]:bg-accent
      [&::-moz-range-thumb]:border-2
      [&::-moz-range-thumb]:border-surface
      [&::-moz-range-thumb]:shadow
      [&::-moz-range-thumb]:cursor-pointer
    "
  />

  <!-- Labels below -->
  <div class="flex justify-between text-[10px] text-muted mt-6 px-0.5">
    <span>{min}</span>
    <span class="font-medium text-accent">{safeMin} – {safeMax}</span>
    <span>{max}</span>
  </div>
</div>
