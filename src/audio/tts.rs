//! Thin wrapper around the `tts` crate (#28).
//!
//! The crate supports macOS (AVFoundation), Linux (Speech Dispatcher),
//! Windows (WinRT), and a few others. The wrapper's job is:
//!
//! 1. Build a single engine for the run via [`TtsEngine::new`].
//! 2. Pick a voice that matches the current `Language` so JA prompts are
//!    read with a Japanese voice and EN prompts with an English voice
//!    even when both are installed on the system.
//! 3. Expose `speak` / `stop` / `is_speaking` in terms type-globe needs;
//!    every call interrupts whatever is currently speaking, so
//!    `Space`-replay (#30) just calls `speak` again.
//!
//! Initialisation may legitimately fail on systems without a TTS daemon
//! running (most often a Linux box without `speech-dispatcher`); the
//! caller surfaces that to the player as "audio unavailable, listening
//! mode disabled" rather than crashing the whole binary.

use crate::types::Language;
use tts::{Tts, Voice};

/// Run-scoped TTS handle. Kept on the main thread (the underlying `Tts`
/// wraps an `Rc` and is therefore `!Send`).
pub struct TtsEngine {
    inner: Tts,
}

impl TtsEngine {
    /// Build the platform default backend. Fails when no backend can be
    /// initialised (e.g. Linux without speech-dispatcher running).
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let inner = Tts::default()?;
        Ok(Self { inner })
    }

    /// Speak `text` using a voice that matches `lang`. If no matching
    /// voice is installed the system default voice is used — better to
    /// hear the prompt in the wrong accent than to fall silent.
    ///
    /// `interrupt = true` so a Space-mash replay flow (#30) does not
    /// queue identical utterances; each call replaces the in-flight one.
    pub fn speak(&mut self, text: &str, lang: &Language) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(voice) = pick_voice(&self.inner, lang) {
            // Voice selection is best-effort — a backend that doesn't
            // support `set_voice` (or rejects this voice) shouldn't kill
            // the run; we still want to attempt the speak call.
            let _ = self.inner.set_voice(&voice);
        }
        self.inner.speak(text, true)?;
        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.inner.stop()?;
        Ok(())
    }

    /// Whether the backend is currently producing audio. Reserved for
    /// the run-loop work in #32-#37 (which will gate Space-replay on
    /// "is the previous utterance still going?"). Foundation flow
    /// always interrupts on replay so this isn't called yet.
    #[allow(dead_code)]
    pub fn is_speaking(&self) -> bool {
        self.inner.is_speaking().unwrap_or(false)
    }
}

/// Best-match voice for `lang`. Compares only the primary subtag (`ja`
/// / `en`) so any locale flavour (`ja-JP`, `en-US`, `en-GB`...) counts
/// as a match. Returns `None` when no voice's primary subtag matches —
/// the caller then falls back to the backend default.
fn pick_voice(tts: &Tts, lang: &Language) -> Option<Voice> {
    let target = lang.code();
    let voices = tts.voices().ok()?;
    voices
        .into_iter()
        .find(|v| v.language().primary_language() == target)
}

#[cfg(test)]
mod tests {
    // We can't construct a real `Tts` in CI (no audio daemon), so the
    // unit tests live in `game::listening` where the blind-input judge
    // is pure. This module is intentionally test-light: integration
    // testing for actual audio is manual on macOS / Linux.

    #[test]
    fn module_compiles() {}
}
