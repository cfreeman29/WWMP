#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod config;
mod keyboard;
mod mapper;
mod midi;
mod playback;

use anyhow::Result;
use tauri::State;
use std::sync::Mutex;

use crate::midi::MidiFile;
use crate::playback::PlaybackEngine;
use crate::config::AppConfig;

pub struct AppState {
    pub config: Mutex<AppConfig>,
    pub midi_file: Mutex<Option<MidiFile>>,
    pub playback: Mutex<PlaybackEngine>,
}

#[tauri::command]
fn load_midi_file(path: String, state: State<AppState>) -> Result<midi::MidiInfo, String> {
    let midi_file = midi::load_file(&path).map_err(|e| e.to_string())?;
    let info = midi_file.info();
    *state.midi_file.lock().unwrap() = Some(midi_file);
    Ok(info)
}

#[tauri::command]
fn play(state: State<AppState>) -> Result<(), String> {
    let midi_file = state.midi_file.lock().unwrap();
    let config = state.config.lock().unwrap();

    if let Some(ref midi) = *midi_file {
        let mut playback = state.playback.lock().unwrap();
        playback.start(midi, &config).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn pause(state: State<AppState>) -> Result<(), String> {
    let mut playback = state.playback.lock().unwrap();
    playback.pause();
    Ok(())
}

#[tauri::command]
fn stop(state: State<AppState>) -> Result<(), String> {
    let mut playback = state.playback.lock().unwrap();
    playback.stop();
    Ok(())
}

#[tauri::command]
fn set_tempo(factor: f64, state: State<AppState>) -> Result<(), String> {
    let mut config = state.config.lock().unwrap();
    config.tempo_factor = factor;
    Ok(())
}

#[tauri::command]
fn set_transpose(semitones: i32, state: State<AppState>) -> Result<(), String> {
    let mut config = state.config.lock().unwrap();
    config.transpose = semitones;
    Ok(())
}

#[tauri::command]
fn get_config(state: State<AppState>) -> AppConfig {
    state.config.lock().unwrap().clone()
}

#[tauri::command]
fn test_key(key: String, modifier: String) -> Result<(), String> {
    let mod_type = match modifier.as_str() {
        "shift" => keyboard::Modifier::Shift,
        "ctrl" => keyboard::Modifier::Ctrl,
        _ => keyboard::Modifier::None,
    };

    keyboard::press_key(&key, mod_type).map_err(|e| e.to_string())?;
    std::thread::sleep(std::time::Duration::from_millis(50));
    keyboard::release_key(&key, mod_type).map_err(|e| e.to_string())?;

    Ok(())
}

fn main() {
    let config = AppConfig::load().unwrap_or_default();

    let app_state = AppState {
        config: Mutex::new(config),
        midi_file: Mutex::new(None),
        playback: Mutex::new(PlaybackEngine::new()),
    };

    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            load_midi_file,
            play,
            pause,
            stop,
            set_tempo,
            set_transpose,
            get_config,
            test_key,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
