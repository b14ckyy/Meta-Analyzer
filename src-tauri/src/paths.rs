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

use std::path::PathBuf;

use tauri::{Manager, Runtime};

/// Resolve the app's data root.
///
/// **Portable mode:** if a `data` folder sits next to the executable, everything
/// (settings, profiles, content_rules, vocabulary, …) is stored there — the app
/// is fully self-contained and leaves no trace in the user profile. This is how
/// the standalone build ships.
///
/// **Installed mode:** otherwise the per-user app-data dir is used
/// (`%APPDATA%/<identifier>`). This is how the installer build behaves.
///
/// The distinction is purely the presence of the `data` folder next to the exe,
/// so the same binary serves both distributions.
pub fn data_dir<R: Runtime, M: Manager<R>>(manager: &M) -> tauri::Result<PathBuf> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let portable = parent.join("data");
            if portable.is_dir() {
                return Ok(portable);
            }
        }
    }
    manager.path().app_data_dir()
}
