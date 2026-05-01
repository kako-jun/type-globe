//! Sound effect cues for Quiz mode (Issue #73).
//!
//! All effects are **synthesized at runtime** as PCM samples; no asset
//! files ship with the binary. The synthesizer is intentionally tiny
//! (sine + linear envelope) so the cargo footprint of `rodio` is the
//! only cost we pay for sound.
//!
//! If audio output is unavailable (no device, sandboxed environment),
//! `CueEngine::new` falls back to a silent stub. Quiz mode is fully
//! playable without sound — the cues are decoration, not signal.

use rodio::buffer::SamplesBuffer;
use rodio::{OutputStream, OutputStreamHandle, Sink};
use std::sync::Mutex;

const SAMPLE_RATE: u32 = 44_100;

/// One of the canned cues a Quiz session can fire. Issue #73 names the
/// "ダダン" question reveal, the "ピンポーン" correct cue, the "ブブー"
/// wrong cue, and the muted typing / mistype clicks.
#[derive(Debug, Clone, Copy)]
pub enum Cue {
    QuestionReveal,
    Correct,
    Wrong,
    Keystroke,
    Mistype,
}

/// Owns the rodio output stream and a single sink for cue playback.
/// Cues are appended; long cues that overlap a fast keystroke are
/// allowed to mix with a fresh sink so the keystroke isn't queued
/// behind a "ピンポーン" tail.
pub struct CueEngine {
    /// Held to keep the audio device open. Never read after construction.
    _stream: OutputStream,
    handle: OutputStreamHandle,
    /// Long-running sink for the rare overlapping case. Wrapped in a
    /// `Mutex` so the engine stays `Sync` for use behind `&self` calls.
    sink: Mutex<Option<Sink>>,
}

impl CueEngine {
    /// Open the default audio device. Returns `None` when no device is
    /// available (CI, headless servers, sandboxes) so callers can fall
    /// back to silent operation without a panic.
    pub fn new() -> Option<Self> {
        let (stream, handle) = OutputStream::try_default().ok()?;
        Some(Self {
            _stream: stream,
            handle,
            sink: Mutex::new(None),
        })
    }

    /// Fire-and-forget playback. Drops silently if a sink can't be
    /// created — sound is decorative, not load-bearing.
    pub fn play(&self, cue: Cue) {
        let samples = synthesize(cue);
        let buffer = SamplesBuffer::new(1, SAMPLE_RATE, samples);
        if let Ok(sink) = Sink::try_new(&self.handle) {
            sink.append(buffer);
            sink.detach();
        }
        // Keep one sink around so repeated short cues share allocations
        // when they don't need to overlap (typing).
        if let Ok(mut guard) = self.sink.lock() {
            // MSRV is 1.78; `Option::is_none_or` is 1.82, so spell it out.
            let needs_new = match guard.as_ref() {
                None => true,
                Some(s) => s.empty(),
            };
            if needs_new {
                if let Ok(new_sink) = Sink::try_new(&self.handle) {
                    *guard = Some(new_sink);
                }
            }
        }
    }
}

/// Render `cue` to a mono PCM buffer at `SAMPLE_RATE`. Each cue mixes a
/// sequence of tone segments with a linear attack/decay envelope so the
/// output reads as a clean cue rather than a square click.
fn synthesize(cue: Cue) -> Vec<f32> {
    match cue {
        // "ダダン" — two short low-mid hits, second slightly higher.
        Cue::QuestionReveal => mix_segments(&[
            tone(330.0, 0.10, 0.35),
            silence(0.05),
            tone(440.0, 0.18, 0.40),
        ]),
        // "ピンポーン" — bright two-note major-third descending arpeggio.
        Cue::Correct => mix_segments(&[
            tone(880.0, 0.18, 0.32),
            silence(0.04),
            tone(660.0, 0.30, 0.32),
        ]),
        // "ブブー" — low buzz, two pulses on the same low pitch.
        Cue::Wrong => mix_segments(&[
            tone(180.0, 0.18, 0.40),
            silence(0.05),
            tone(180.0, 0.20, 0.40),
        ]),
        // Quiet typing tick — short, mid-high, low amplitude.
        Cue::Keystroke => tone(720.0, 0.025, 0.05),
        // Mistype — slightly lower, slightly louder, shorter than "wrong"
        // so it doesn't overshadow the question feedback.
        Cue::Mistype => tone(220.0, 0.05, 0.12),
    }
}

fn mix_segments(segments: &[Vec<f32>]) -> Vec<f32> {
    let total: usize = segments.iter().map(|s| s.len()).sum();
    let mut out = Vec::with_capacity(total);
    for seg in segments {
        out.extend_from_slice(seg);
    }
    out
}

fn silence(duration_s: f32) -> Vec<f32> {
    let samples = (duration_s * SAMPLE_RATE as f32) as usize;
    vec![0.0; samples]
}

/// One tone segment: sine wave at `freq` Hz, `duration_s` seconds, peak
/// amplitude `gain` in `[0.0, 1.0]`. A short attack/decay envelope avoids
/// the click that a hard-edged buffer would otherwise produce.
fn tone(freq: f32, duration_s: f32, gain: f32) -> Vec<f32> {
    let total = (duration_s * SAMPLE_RATE as f32) as usize;
    let attack = (total / 20).max(1);
    let release = (total / 8).max(1);
    let mut buf = Vec::with_capacity(total);
    for i in 0..total {
        let t = i as f32 / SAMPLE_RATE as f32;
        let env = if i < attack {
            i as f32 / attack as f32
        } else if i + release > total {
            (total - i) as f32 / release as f32
        } else {
            1.0
        };
        let s = (2.0 * std::f32::consts::PI * freq * t).sin() * gain * env;
        buf.push(s);
    }
    buf
}
