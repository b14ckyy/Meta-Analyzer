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

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, LogicalSize, Manager, Runtime, Window};

#[derive(Debug, Serialize, Deserialize)]
struct WindowSize {
    width: f64,
    height: f64,
}

/// Restore saved window size, or use the Tauri config default (1400x900) on first launch.
pub fn restore_window_size(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let window = app.get_webview_window("main").ok_or("main window not found")?;

    let size_path = crate::paths::data_dir(app)
        .map_err(|e| format!("app data dir: {}", e))?
        .join("window_size.json");

    if size_path.exists() {
        let text = std::fs::read_to_string(&size_path)?;
        if let Ok(ws) = serde_json::from_str::<WindowSize>(&text) {
            // Clamp to reasonable bounds
            let w = ws.width.clamp(800.0, 4000.0);
            let h = ws.height.clamp(600.0, 4000.0);
            let _ = window.set_size(LogicalSize::new(w, h));
        }
    } else {
        // Only center on very first launch (no saved size)
        let _ = window.center();
    }

    Ok(())
}

/// Save current window inner size on close, so it can be restored next launch.
pub fn save_window_size<R: Runtime>(window: &Window<R>) {
    let app = window.app_handle();
    let size_path = match crate::paths::data_dir(app) {
        Ok(d) => d.join("window_size.json"),
        Err(_) => return,
    };

    if let Ok(size) = window.inner_size() {
        let ws = WindowSize {
            width: size.width as f64,
            height: size.height as f64,
        };
        if let Some(parent) = size_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(text) = serde_json::to_string(&ws) {
            let _ = std::fs::write(size_path, text);
        }
    }
}
