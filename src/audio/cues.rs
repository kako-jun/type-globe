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

/// Owns the rodio output stream used to play synthesized cues. Each
/// `play` call hands a freshly built `Sink` to a background thread via
/// `Sink::detach`, so two cues that overlap (a keystroke during the
/// "ピンポーン" tail) mix at the device level instead of queueing.
pub struct CueEngine {
    /// Held to keep the audio device open. The handle below is what
    /// playback actually goes through; `_stream` is kept to anchor the
    /// device's lifetime to the engine's.
    #[allow(dead_code)]
    _stream: OutputStream,
    handle: OutputStreamHandle,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tone_sample_count_matches_duration() {
        let buf = tone(440.0, 0.10, 0.5);
        let expected = (0.10 * SAMPLE_RATE as f32) as usize;
        assert_eq!(buf.len(), expected);
    }

    #[test]
    fn tone_amplitude_bounded_by_gain() {
        let gain = 0.3;
        let buf = tone(440.0, 0.05, gain);
        let max = buf.iter().cloned().fold(0.0_f32, f32::max);
        let min = buf.iter().cloned().fold(0.0_f32, f32::min);
        assert!(max <= gain + f32::EPSILON);
        assert!(min >= -gain - f32::EPSILON);
    }

    #[test]
    fn tone_envelope_grows_during_attack_and_settles_in_sustain() {
        // 200 ms tone gives a wide enough attack window to sample the
        // envelope reliably without sine-wave interference.
        let buf = tone(440.0, 0.20, 0.5);
        let attack = (buf.len() / 20).max(1);
        // Compare the early part of the attack against the late part of
        // the attack; the envelope must have ramped up.
        let early_peak = buf[..attack / 4]
            .iter()
            .map(|s| s.abs())
            .fold(0.0_f32, f32::max);
        let late_peak = buf[attack / 4 * 3..attack]
            .iter()
            .map(|s| s.abs())
            .fold(0.0_f32, f32::max);
        assert!(
            late_peak > early_peak,
            "attack should ramp up: early={early_peak} late={late_peak}"
        );
        // Sustain region (well past the attack, before release) should
        // hit close to the gain ceiling.
        let release = (buf.len() / 8).max(1);
        let sustain_start = attack;
        let sustain_end = buf.len().saturating_sub(release);
        if sustain_end > sustain_start {
            let sustain_peak = buf[sustain_start..sustain_end]
                .iter()
                .map(|s| s.abs())
                .fold(0.0_f32, f32::max);
            assert!(sustain_peak > 0.4, "sustain peak too low: {sustain_peak}");
        }
    }

    #[test]
    fn silence_is_all_zero() {
        let buf = silence(0.05);
        assert!(buf.iter().all(|&s| s == 0.0));
        assert_eq!(buf.len(), (0.05 * SAMPLE_RATE as f32) as usize);
    }

    #[test]
    fn synthesize_question_reveal_is_two_segments_with_a_gap() {
        let buf = synthesize(Cue::QuestionReveal);
        // 0.10 + 0.05 + 0.18 seconds total at SAMPLE_RATE.
        let expected = ((0.10 + 0.05 + 0.18) * SAMPLE_RATE as f32) as usize;
        // Allow a 1-sample rounding tolerance from three independent casts.
        assert!(
            buf.len().abs_diff(expected) <= 3,
            "len={} expected={}",
            buf.len(),
            expected
        );
    }

    #[test]
    fn synthesize_keystroke_is_short_and_quiet() {
        let buf = synthesize(Cue::Keystroke);
        let expected = (0.025 * SAMPLE_RATE as f32) as usize;
        assert_eq!(buf.len(), expected);
        let peak = buf.iter().map(|s| s.abs()).fold(0.0_f32, f32::max);
        assert!(peak <= 0.05 + f32::EPSILON);
    }

    #[test]
    fn synthesize_mistype_is_louder_than_keystroke() {
        let key = synthesize(Cue::Keystroke);
        let miss = synthesize(Cue::Mistype);
        let key_peak = key.iter().map(|s| s.abs()).fold(0.0_f32, f32::max);
        let miss_peak = miss.iter().map(|s| s.abs()).fold(0.0_f32, f32::max);
        assert!(
            miss_peak > key_peak,
            "mistype peak {miss_peak} should exceed keystroke peak {key_peak}"
        );
    }
}
