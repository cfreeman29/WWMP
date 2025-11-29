use crate::config::AppConfig;
use crate::keyboard::Modifier;

/// Represents an octave in the game instrument
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Octave {
    Low,
    Medium,
    High,
}

/// Represents an accidental (sharp/flat/natural)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Accidental {
    Flat,    // -1 semitone, Ctrl modifier
    Natural, // 0, no modifier
    Sharp,   // +1 semitone, Shift modifier
}

/// A note that can be played on the in-game instrument
#[derive(Debug, Clone)]
pub struct InstrumentNote {
    pub octave: Octave,
    pub degree: u8,        // 1-7
    pub accidental: Accidental,
}

/// A keystroke to send to the game
#[derive(Debug, Clone)]
pub struct KeyStroke {
    pub key: String,
    pub modifier: Modifier,
}

impl Accidental {
    pub fn to_modifier(self) -> Modifier {
        match self {
            Accidental::Flat => Modifier::Ctrl,
            Accidental::Natural => Modifier::None,
            Accidental::Sharp => Modifier::Shift,
        }
    }
}

/// Semitone offsets for each scale degree in a major scale
const DEGREE_SEMITONES: [i32; 7] = [0, 2, 4, 5, 7, 9, 11];

/// Map a MIDI note to an instrument note
/// Returns None if the note is out of range
pub fn midi_to_instrument(midi_note: u8, config: &AppConfig) -> Option<InstrumentNote> {
    let transposed = midi_note as i32 + config.transpose;
    let reference = config.reference_midi_note as i32;

    // Calculate semitones from reference (Medium octave, degree 1)
    let semitones_from_ref = transposed - reference;

    // Calculate octave offset and position within octave
    let octave_offset = semitones_from_ref.div_euclid(12);
    let within_octave = semitones_from_ref.rem_euclid(12) as u8;

    // Find the best matching degree and accidental
    let (degree, accidental) = find_degree_and_accidental(within_octave)?;

    // Calculate final octave (Medium + offset)
    let octave = match octave_offset {
        -1 => Octave::Low,
        0 => Octave::Medium,
        1 => Octave::High,
        _ => return None, // Out of range
    };

    Some(InstrumentNote {
        octave,
        degree,
        accidental,
    })
}

/// Find the scale degree and accidental for a given semitone position within an octave
fn find_degree_and_accidental(semitones: u8) -> Option<(u8, Accidental)> {
    // Check for exact match (natural note)
    for (i, &deg_semi) in DEGREE_SEMITONES.iter().enumerate() {
        if deg_semi as u8 == semitones {
            return Some((i as u8 + 1, Accidental::Natural));
        }
    }

    // Check for sharp (degree + 1 semitone)
    for (i, &deg_semi) in DEGREE_SEMITONES.iter().enumerate() {
        if (deg_semi + 1) as u8 == semitones {
            return Some((i as u8 + 1, Accidental::Sharp));
        }
    }

    // Check for flat (degree - 1 semitone)
    for (i, &deg_semi) in DEGREE_SEMITONES.iter().enumerate() {
        if deg_semi > 0 && (deg_semi - 1) as u8 == semitones {
            return Some((i as u8 + 1, Accidental::Flat));
        }
    }

    // Edge case: semitone 11 could be flat of degree 1 in next octave
    // But we handle this at the octave level instead

    None
}

/// Convert an instrument note to a keystroke
pub fn note_to_keystroke(note: &InstrumentNote, config: &AppConfig) -> Option<KeyStroke> {
    let keys = match note.octave {
        Octave::High => &config.key_mapping.high,
        Octave::Medium => &config.key_mapping.medium,
        Octave::Low => &config.key_mapping.low,
    };

    let index = (note.degree - 1) as usize;
    if index >= keys.len() {
        return None;
    }

    Some(KeyStroke {
        key: keys[index].clone(),
        modifier: note.accidental.to_modifier(),
    })
}

/// Analyze MIDI note range and suggest optimal transpose value
pub fn suggest_transpose(midi_notes: &[u8], reference: u8) -> i32 {
    if midi_notes.is_empty() {
        return 0;
    }

    let min_note = *midi_notes.iter().min().unwrap() as i32;
    let max_note = *midi_notes.iter().max().unwrap() as i32;
    let ref_note = reference as i32;

    // Playable range: Low octave degree 1 to High octave degree 7
    // That's reference - 12 to reference + 23 (roughly 3 octaves)
    let playable_min = ref_note - 12;
    let playable_max = ref_note + 23;

    // Try different transpose values to find optimal fit
    let mut best_transpose = 0;
    let mut best_out_of_range = i32::MAX;

    for transpose in (-24..=24).step_by(12) {
        let t_min = min_note + transpose;
        let t_max = max_note + transpose;

        let out_of_range = (playable_min - t_min).max(0) + (t_max - playable_max).max(0);

        if out_of_range < best_out_of_range {
            best_out_of_range = out_of_range;
            best_transpose = transpose;
        }
    }

    best_transpose
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_degree_semitones() {
        // C major scale: C=0, D=2, E=4, F=5, G=7, A=9, B=11
        assert_eq!(find_degree_and_accidental(0), Some((1, Accidental::Natural)));
        assert_eq!(find_degree_and_accidental(2), Some((2, Accidental::Natural)));
        assert_eq!(find_degree_and_accidental(4), Some((3, Accidental::Natural)));
        assert_eq!(find_degree_and_accidental(5), Some((4, Accidental::Natural)));
        assert_eq!(find_degree_and_accidental(7), Some((5, Accidental::Natural)));
        assert_eq!(find_degree_and_accidental(9), Some((6, Accidental::Natural)));
        assert_eq!(find_degree_and_accidental(11), Some((7, Accidental::Natural)));
    }

    #[test]
    fn test_sharps() {
        // C# = 1, D# = 3, F# = 6, G# = 8, A# = 10
        assert_eq!(find_degree_and_accidental(1), Some((1, Accidental::Sharp)));
        assert_eq!(find_degree_and_accidental(3), Some((2, Accidental::Sharp)));
        assert_eq!(find_degree_and_accidental(6), Some((4, Accidental::Sharp)));
        assert_eq!(find_degree_and_accidental(8), Some((5, Accidental::Sharp)));
        assert_eq!(find_degree_and_accidental(10), Some((6, Accidental::Sharp)));
    }
}
