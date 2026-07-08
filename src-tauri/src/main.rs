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

// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod content_types;
mod models;
mod paths;
mod processing;
mod video_models;
mod window_state;

use processing::queue::ProcessingQueue;
use processing::video_queue::VideoProcessingQueue;
use tauri::Manager;

/// Seed the editable default content rules into the app data directory.
///
/// General and Custom are compiled into the binary (not seeded here). Every
/// other category ships as an editable `image_*` / `video_*` JSON file that is
/// copied into `{app_data}/content_rules/` on first run, so users can edit them
/// and drop in their own packs. Existing files are never overwritten. Optional
/// add-on packs are not seeded — they ship separately and are dropped in by the
/// user.
fn copy_content_rules(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let app_data_dir = crate::paths::data_dir(app)?;
    let rules_dir = app_data_dir.join("content_rules");
    std::fs::create_dir_all(&rules_dir)?;

    let default_rules: &[&str] = &[
        // Photo categories
        "image_holidays_events",
        "image_family_kids",
        "image_pets_animals",
        "image_nature_travel",
        "image_food_cooking",
        "image_sports_fitness",
        "image_architecture_interiors",
        "image_fashion_style",
        "image_cars_vehicles",
        "image_music_concerts",
        "image_art_culture",
        "image_business_work",
        // Video categories
        "video_travel",
        "video_parties",
        "video_nature",
        "video_tech",
        "video_sports",
        "video_music",
        "video_cars",
        "video_cooking",
        "video_family",
        "video_pets",
        "video_gaming",
        "video_tutorial",
    ];

    // Candidate source directories, in priority order:
    //  1. the bundled resource dir (installed app)
    //  2. a content_rules folder next to the executable (portable standalone .exe)
    //  3. the repo's content_rules dir (dev builds — path baked at compile time)
    let mut sources: Vec<std::path::PathBuf> = Vec::new();
    if let Ok(resource_dir) = app.path().resource_dir() {
        sources.push(resource_dir.join("content_rules"));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            sources.push(dir.join("content_rules"));
        }
    }
    #[cfg(debug_assertions)]
    sources.push(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("content_rules"));

    for slug in default_rules {
        let dest_path = rules_dir.join(format!("{}.json", slug));
        if dest_path.exists() {
            continue; // Don't overwrite user-modified rules
        }
        for src_dir in &sources {
            let src = src_dir.join(format!("{}.json", slug));
            if src.exists() {
                std::fs::copy(&src, &dest_path)?;
                eprintln!("[setup] Seeded content rule: {:?} -> {:?}", src, dest_path);
                break;
            }
        }
    }

    Ok(())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let handle = app.handle();
            window_state::restore_window_size(handle)?;

            if let Err(e) = copy_content_rules(app) {
                eprintln!("[setup] Warning: could not copy content rules: {}", e);
            }

            // Temporäre Verzeichnisse aus vorherigen Sitzungen bereinigen
            // (z.B. nach einem Absturz). Reasoning-Dateien werden hier gelöscht,
            // aber während der aktiven Session bleiben sie erhalten.
            processing::video_queue::cleanup_temp_dirs();

            Ok(())
        })
        .manage(ProcessingQueue::new())
        .manage(VideoProcessingQueue::new())
        .invoke_handler(tauri::generate_handler![
            commands::dialog::pick_folder,
            commands::dialog::pick_files,
            commands::dialog::resolve_media_paths,
            commands::dialog::scan_folder_for_images,
            commands::dialog::read_images_tags,
            commands::dialog::fetch_available_models,
            commands::processor::start_processing,
            commands::processor::apply_photo_metadata,
            commands::processor::pause_processing,
            commands::processor::resume_processing,
            commands::processor::stop_processing,
            commands::settings::load_settings,
            commands::settings::save_settings,
            commands::settings::list_profiles,
            commands::settings::load_profile,
            commands::settings::save_profile,
            commands::settings::delete_profile,
            commands::settings::load_vocabulary,
            commands::settings::preview_prompt,
            commands::video_processor::start_video_processing,
            commands::video_processor::stop_video_processing,
            commands::video_processor::pause_video_processing,
            commands::video_processor::resume_video_processing,
            commands::video_processor::apply_video_metadata,
            commands::video_processor::scan_folder_for_videos,
            commands::video_processor::get_video_duration,
            commands::video_processor::create_video_job,
            commands::video_processor::create_video_thumbnail,
            // Video Profile Commands
            commands::settings::list_video_profiles,
            commands::settings::load_video_profile,
            commands::settings::save_video_profile,
            commands::settings::delete_video_profile,
            commands::settings::load_video_vocabulary,
            commands::settings::preview_video_prompt,
            commands::settings::list_content_types,
            commands::settings::list_video_content_types,
            commands::settings::open_content_rules_dir,
            commands::settings::open_file,
            commands::settings::reveal_in_explorer,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                window_state::save_window_size(&window);
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
