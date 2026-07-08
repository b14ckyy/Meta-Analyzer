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
  import type { JobStatus } from '../lib/types';

  interface Props {
    status: JobStatus;
  }

  let { status }: Props = $props();

  const styles: Record<JobStatus, string> = {
    pending:    'bg-surface2 text-muted',
    processing: 'bg-accent/10 text-accent',
    done:       'bg-success/10 text-success',
    error:      'bg-danger/10 text-danger',
    skipped:    'bg-surface2 text-muted',
  };

  let style = $derived(styles[status]);
  let label = $derived(status.charAt(0).toUpperCase() + status.slice(1));
</script>

<span class={`px-2 py-1 rounded-lg text-sm font-medium ${style}`}>{label}</span>
