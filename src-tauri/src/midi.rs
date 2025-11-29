use anyhow::Result;
use midly::{Smf, Timing, TrackEventKind, MidiMessage};
use serde::{Deserialize, Serialize};
use std::fs;

/// Information about a loaded MIDI file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MidiInfo {
    pub track_count: usize,
    pub duration_ms: u64,
    pub note_count: usize,
    pub min_note: u8,
    pub max_note: u8,
}

/// A single note event with timing
#[derive(Debug, Clone)]
pub struct NoteEvent {
    pub start_ms: u64,
    pub duration_ms: u64,
    pub note: u8,
    pub velocity: u8,
}

/// Represents a loaded and processed MIDI file
#[derive(Debug)]
pub struct MidiFile {
    pub info: MidiInfo,
    pub events: Vec<NoteEvent>,
}

impl MidiFile {
    pub fn info(&self) -> MidiInfo {
        self.info.clone()
    }
}

/// Load and parse a MIDI file
pub fn load_file(path: &str) -> Result<MidiFile> {
    let data = fs::read(path)?;
    let smf = Smf::parse(&data)?;

    let ticks_per_beat = match smf.header.timing {
        Timing::Metrical(tpb) => tpb.as_int() as u32,
        Timing::Timecode(fps, sub) => (fps.as_f32() * sub as f32) as u32,
    };

    // Build tempo map (microseconds per beat at each tick)
    let tempo_map = build_tempo_map(&smf);

    // Extract all note events
    let mut events = Vec::new();
    let mut pending_notes: Vec<(u8, u64, u8)> = Vec::new(); // (note, start_ms, velocity)

    for track in &smf.tracks {
        let mut current_tick: u32 = 0;

        for event in track {
            current_tick += event.delta.as_int();
            let current_ms = ticks_to_ms(current_tick, ticks_per_beat, &tempo_map);

            if let TrackEventKind::Midi { message, .. } = event.kind {
                match message {
                    MidiMessage::NoteOn { key, vel } => {
                        let note = key.as_int();
                        let velocity = vel.as_int();

                        if velocity > 0 {
                            // Note on
                            pending_notes.push((note, current_ms, velocity));
                        } else {
                            // Note off (velocity 0)
                            finish_note(&mut pending_notes, &mut events, note, current_ms);
                        }
                    }
                    MidiMessage::NoteOff { key, .. } => {
                        let note = key.as_int();
                        finish_note(&mut pending_notes, &mut events, note, current_ms);
                    }
                    _ => {}
                }
            }
        }

        // Close any remaining pending notes at track end
        let track_end_ms = ticks_to_ms(current_tick, ticks_per_beat, &tempo_map);
        for (note, start_ms, velocity) in pending_notes.drain(..) {
            events.push(NoteEvent {
                start_ms,
                duration_ms: track_end_ms.saturating_sub(start_ms),
                note,
                velocity,
            });
        }
    }

    // Sort by start time
    events.sort_by_key(|e| e.start_ms);

    // Calculate stats
    let duration_ms = events.iter().map(|e| e.start_ms + e.duration_ms).max().unwrap_or(0);
    let min_note = events.iter().map(|e| e.note).min().unwrap_or(0);
    let max_note = events.iter().map(|e| e.note).max().unwrap_or(127);

    let info = MidiInfo {
        track_count: smf.tracks.len(),
        duration_ms,
        note_count: events.len(),
        min_note,
        max_note,
    };

    Ok(MidiFile { info, events })
}

fn finish_note(
    pending: &mut Vec<(u8, u64, u8)>,
    events: &mut Vec<NoteEvent>,
    note: u8,
    end_ms: u64,
) {
    if let Some(idx) = pending.iter().position(|(n, _, _)| *n == note) {
        let (note, start_ms, velocity) = pending.remove(idx);
        events.push(NoteEvent {
            start_ms,
            duration_ms: end_ms.saturating_sub(start_ms),
            note,
            velocity,
        });
    }
}

/// Build a tempo map: Vec of (tick, microseconds_per_beat)
fn build_tempo_map(smf: &Smf) -> Vec<(u32, u32)> {
    let mut tempo_map = vec![(0u32, 500_000u32)]; // Default: 120 BPM

    for track in &smf.tracks {
        let mut current_tick: u32 = 0;

        for event in track {
            current_tick += event.delta.as_int();

            if let TrackEventKind::Meta(midly::MetaMessage::Tempo(tempo)) = event.kind {
                tempo_map.push((current_tick, tempo.as_int()));
            }
        }
    }

    tempo_map.sort_by_key(|(tick, _)| *tick);
    tempo_map
}

/// Convert ticks to milliseconds using the tempo map
fn ticks_to_ms(tick: u32, ticks_per_beat: u32, tempo_map: &[(u32, u32)]) -> u64 {
    let mut ms: f64 = 0.0;
    let mut prev_tick: u32 = 0;
    let mut current_tempo: u32 = 500_000; // Default 120 BPM

    for &(tempo_tick, tempo) in tempo_map {
        if tempo_tick >= tick {
            break;
        }

        // Add time for ticks in previous tempo region
        let delta_ticks = tempo_tick.saturating_sub(prev_tick);
        ms += (delta_ticks as f64 * current_tempo as f64) / (ticks_per_beat as f64 * 1000.0);

        prev_tick = tempo_tick;
        current_tempo = tempo;
    }

    // Add remaining ticks at current tempo
    let delta_ticks = tick.saturating_sub(prev_tick);
    ms += (delta_ticks as f64 * current_tempo as f64) / (ticks_per_beat as f64 * 1000.0);

    ms as u64
}

/// Apply polyphony limit to events at similar timestamps
pub fn limit_polyphony(events: &mut Vec<NoteEvent>, max_notes: usize, tolerance_ms: u64) {
    if max_notes == 0 || events.is_empty() {
        return;
    }

    // Group events by approximate start time
    let mut i = 0;
    while i < events.len() {
        let start = events[i].start_ms;
        let mut group_end = i;

        // Find all events within tolerance
        while group_end + 1 < events.len()
            && events[group_end + 1].start_ms <= start + tolerance_ms
        {
            group_end += 1;
        }

        // If group exceeds max polyphony, keep only highest notes
        let group_size = group_end - i + 1;
        if group_size > max_notes {
            // Sort group by note (descending) and keep top N
            let mut group: Vec<_> = events[i..=group_end].to_vec();
            group.sort_by(|a, b| b.note.cmp(&a.note));
            group.truncate(max_notes);

            // Replace in events
            events.splice(i..=group_end, group);
            i += max_notes;
        } else {
            i = group_end + 1;
        }
    }
}
