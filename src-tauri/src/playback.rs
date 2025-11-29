use anyhow::Result;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use crate::config::AppConfig;
use crate::keyboard::{self, Modifier};
use crate::mapper::{midi_to_instrument, note_to_keystroke};
use crate::midi::{limit_polyphony, MidiFile, NoteEvent};

/// Scheduled keystroke event
#[derive(Debug, Clone)]
struct ScheduledEvent {
    time_ms: u64,
    key: String,
    modifier: Modifier,
    is_key_down: bool,
}

/// Playback engine state
#[derive(Debug)]
pub struct PlaybackEngine {
    is_playing: Arc<AtomicBool>,
    is_paused: Arc<AtomicBool>,
}

impl PlaybackEngine {
    pub fn new() -> Self {
        Self {
            is_playing: Arc::new(AtomicBool::new(false)),
            is_paused: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Start playback of the MIDI file
    pub fn start(&mut self, midi: &MidiFile, config: &AppConfig) -> Result<()> {
        // Stop any existing playback
        self.stop();

        // Build event timeline
        let events = build_timeline(midi, config)?;
        if events.is_empty() {
            return Ok(());
        }

        let is_playing = self.is_playing.clone();
        let is_paused = self.is_paused.clone();
        let start_delay = config.start_delay_ms;
        let tempo_factor = config.tempo_factor;

        is_playing.store(true, Ordering::SeqCst);
        is_paused.store(false, Ordering::SeqCst);

        // Spawn playback thread
        thread::spawn(move || {
            let start_time = Instant::now();
            let mut event_index = 0;

            // Initial delay
            thread::sleep(Duration::from_millis(start_delay));

            while event_index < events.len() && is_playing.load(Ordering::SeqCst) {
                // Handle pause
                while is_paused.load(Ordering::SeqCst) && is_playing.load(Ordering::SeqCst) {
                    thread::sleep(Duration::from_millis(10));
                }

                if !is_playing.load(Ordering::SeqCst) {
                    break;
                }

                let elapsed = start_time.elapsed().as_millis() as u64;
                let scaled_elapsed = (elapsed as f64 * tempo_factor) as u64;

                // Process all events that should have fired by now
                while event_index < events.len() {
                    let event = &events[event_index];
                    if event.time_ms > scaled_elapsed {
                        break;
                    }

                    // Fire the event
                    let _ = if event.is_key_down {
                        keyboard::press_key(&event.key, event.modifier)
                    } else {
                        keyboard::release_key(&event.key, event.modifier)
                    };

                    event_index += 1;
                }

                // Small sleep to avoid busy-waiting
                thread::sleep(Duration::from_micros(500));
            }

            // Release all keys when done
            let _ = keyboard::release_all();
            is_playing.store(false, Ordering::SeqCst);
        });

        Ok(())
    }

    /// Pause playback
    pub fn pause(&mut self) {
        if self.is_playing.load(Ordering::SeqCst) {
            let currently_paused = self.is_paused.load(Ordering::SeqCst);
            self.is_paused.store(!currently_paused, Ordering::SeqCst);

            // If pausing, release all keys
            if !currently_paused {
                let _ = keyboard::release_all();
            }
        }
    }

    /// Stop playback
    pub fn stop(&mut self) {
        self.is_playing.store(false, Ordering::SeqCst);
        self.is_paused.store(false, Ordering::SeqCst);
        let _ = keyboard::release_all();
    }

    /// Check if currently playing
    pub fn is_playing(&self) -> bool {
        self.is_playing.load(Ordering::SeqCst)
    }

    /// Check if currently paused
    pub fn is_paused(&self) -> bool {
        self.is_paused.load(Ordering::SeqCst)
    }
}

impl Default for PlaybackEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Build a timeline of keyboard events from MIDI events
fn build_timeline(midi: &MidiFile, config: &AppConfig) -> Result<Vec<ScheduledEvent>> {
    let mut events = midi.events.clone();

    // Apply polyphony limit
    limit_polyphony(&mut events, config.max_polyphony as usize, 10);

    let mut scheduled = Vec::new();

    for note_event in &events {
        // Map MIDI note to instrument note
        let instrument_note = match midi_to_instrument(note_event.note, config) {
            Some(n) => n,
            None => continue, // Skip out-of-range notes
        };

        // Get keystroke for this note
        let keystroke = match note_to_keystroke(&instrument_note, config) {
            Some(k) => k,
            None => continue,
        };

        // Schedule key down
        scheduled.push(ScheduledEvent {
            time_ms: note_event.start_ms,
            key: keystroke.key.clone(),
            modifier: keystroke.modifier,
            is_key_down: true,
        });

        // Schedule key up
        // Use minimum duration of 30ms to ensure the keypress registers
        let duration = note_event.duration_ms.max(30);
        scheduled.push(ScheduledEvent {
            time_ms: note_event.start_ms + duration,
            key: keystroke.key,
            modifier: keystroke.modifier,
            is_key_down: false,
        });
    }

    // Sort by time
    scheduled.sort_by_key(|e| e.time_ms);

    Ok(scheduled)
}
