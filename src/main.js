const { invoke } = window.__TAURI__.tauri;
const { open } = window.__TAURI__.dialog;

// State
let midiLoaded = false;

// DOM Elements
const openFileBtn = document.getElementById('openFile');
const fileNameSpan = document.getElementById('fileName');
const fileInfoDiv = document.getElementById('fileInfo');
const playBtn = document.getElementById('playBtn');
const pauseBtn = document.getElementById('pauseBtn');
const stopBtn = document.getElementById('stopBtn');
const tempoSlider = document.getElementById('tempo');
const tempoValue = document.getElementById('tempoValue');
const transposeSlider = document.getElementById('transpose');
const transposeValue = document.getElementById('transposeValue');
const polyphonySelect = document.getElementById('polyphony');
const delayInput = document.getElementById('delay');
const statusSpan = document.getElementById('status');

// File open handler
openFileBtn.addEventListener('click', async () => {
  try {
    const filePath = await open({
      multiple: false,
      filters: [{
        name: 'MIDI Files',
        extensions: ['mid', 'midi']
      }]
    });

    if (filePath) {
      setStatus('Loading...');
      const info = await invoke('load_midi_file', { path: filePath });

      // Update UI
      const fileName = filePath.split(/[/\\]/).pop();
      fileNameSpan.textContent = fileName;

      document.getElementById('duration').textContent = formatDuration(info.duration_ms);
      document.getElementById('noteCount').textContent = info.note_count.toLocaleString();
      document.getElementById('noteRange').textContent = `${midiNoteToName(info.min_note)} - ${midiNoteToName(info.max_note)}`;
      document.getElementById('trackCount').textContent = info.track_count;

      fileInfoDiv.classList.remove('hidden');
      midiLoaded = true;
      updatePlaybackButtons();
      setStatus('File loaded');
    }
  } catch (e) {
    setStatus(`Error: ${e}`, true);
    console.error(e);
  }
});

// Playback controls
playBtn.addEventListener('click', async () => {
  try {
    await invoke('play');
    setStatus('Playing...');
    playBtn.disabled = true;
    pauseBtn.disabled = false;
    stopBtn.disabled = false;
  } catch (e) {
    setStatus(`Error: ${e}`, true);
  }
});

pauseBtn.addEventListener('click', async () => {
  try {
    await invoke('pause');
    setStatus('Paused');
    playBtn.disabled = false;
  } catch (e) {
    setStatus(`Error: ${e}`, true);
  }
});

stopBtn.addEventListener('click', async () => {
  try {
    await invoke('stop');
    setStatus('Stopped');
    updatePlaybackButtons();
  } catch (e) {
    setStatus(`Error: ${e}`, true);
  }
});

// Settings handlers
tempoSlider.addEventListener('input', async () => {
  const value = tempoSlider.value;
  tempoValue.textContent = `${value}%`;
  try {
    await invoke('set_tempo', { factor: value / 100 });
  } catch (e) {
    console.error(e);
  }
});

transposeSlider.addEventListener('input', async () => {
  const value = transposeSlider.value;
  transposeValue.textContent = value > 0 ? `+${value}` : value;
  try {
    await invoke('set_transpose', { semitones: parseInt(value) });
  } catch (e) {
    console.error(e);
  }
});

// Keyboard test handlers
document.querySelectorAll('.key').forEach(key => {
  key.addEventListener('click', async () => {
    const keyName = key.dataset.key;
    const modifier = key.dataset.mod;

    try {
      await invoke('test_key', { key: keyName, modifier: modifier });
      key.style.background = 'var(--accent)';
      setTimeout(() => {
        key.style.background = '';
      }, 100);
    } catch (e) {
      setStatus(`Key test failed: ${e}`, true);
    }
  });
});

// Helper functions
function updatePlaybackButtons() {
  playBtn.disabled = !midiLoaded;
  pauseBtn.disabled = true;
  stopBtn.disabled = true;
}

function setStatus(message, isError = false) {
  statusSpan.textContent = message;
  statusSpan.style.color = isError ? 'var(--accent)' : 'var(--success)';
}

function formatDuration(ms) {
  const seconds = Math.floor(ms / 1000);
  const minutes = Math.floor(seconds / 60);
  const secs = seconds % 60;
  return `${minutes}:${secs.toString().padStart(2, '0')}`;
}

function midiNoteToName(note) {
  const names = ['C', 'C#', 'D', 'D#', 'E', 'F', 'F#', 'G', 'G#', 'A', 'A#', 'B'];
  const octave = Math.floor(note / 12) - 1;
  const name = names[note % 12];
  return `${name}${octave}`;
}

// Initialize
document.addEventListener('DOMContentLoaded', () => {
  updatePlaybackButtons();
  setStatus('Ready');
});
