// Meta-Analyzer - AI-powered metadata tagger for photos and videos.
// Copyright (C) 2026 b14ckyy
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

import { mount } from 'svelte';
import App from './App.svelte';
import './app.css';
import { appState } from './lib/store.svelte';

// Theme früh anwenden (vor dem Mount), um ein Aufblitzen des falschen Themes zu vermeiden.
appState.initTheme();

console.log('[main.ts] Script loaded, mounting Svelte app...');

const target = document.getElementById('app');
if (!target) {
  document.body.innerHTML = '<pre style="padding:20px;color:red">#app element not found in DOM</pre>';
  throw new Error('#app target not found');
}

try {
  const app = mount(App, { target });
  console.log('[main.ts] Mount successful');
  (window as unknown as { __app: unknown }).__app = app;
} catch (err) {
  console.error('[main.ts] Mount failed:', err);
  target.innerHTML = `<pre style="padding:20px;color:red;white-space:pre-wrap">Mount error: ${String(err)}\n\n${err instanceof Error ? err.stack : ''}</pre>`;
}
