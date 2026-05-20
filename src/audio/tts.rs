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
use tts::{Features, Tts, Voice};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(not(test), allow(dead_code))]
pub enum TtsRequestKind {
    PromptAnswer,
    PromptReplay,
    BossHint { layer: u8 },
    BossReveal,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TtsProfile {
    pub interrupt: bool,
    pub rate_multiplier: f32,
}

impl TtsProfile {
    pub fn for_request(kind: TtsRequestKind) -> Self {
        match kind {
            TtsRequestKind::PromptAnswer => Self {
                interrupt: true,
                rate_multiplier: 1.0,
            },
            TtsRequestKind::PromptReplay => Self {
                interrupt: true,
                rate_multiplier: 0.96,
            },
            TtsRequestKind::BossHint { layer } => Self {
                interrupt: true,
                rate_multiplier: if layer <= 2 {
                    0.88
                } else if layer <= 4 {
                    0.93
                } else {
                    0.98
                },
            },
            TtsRequestKind::BossReveal => Self {
                interrupt: true,
                rate_multiplier: 1.02,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TtsSupportLevel {
    Preferred,
    Basic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TtsRuntimeSupport {
    pub level: TtsSupportLevel,
    pub can_stop: bool,
    pub can_set_rate: bool,
    pub can_choose_voice: bool,
}

impl TtsRuntimeSupport {
    fn from_features(features: Features) -> Self {
        let level = if features.stop && features.rate {
            TtsSupportLevel::Preferred
        } else {
            TtsSupportLevel::Basic
        };
        Self {
            level,
            can_stop: features.stop,
            can_set_rate: features.rate,
            can_choose_voice: features.voice,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TtsRequest<'a> {
    pub text: &'a str,
    pub lang: &'a Language,
    pub kind: TtsRequestKind,
}

/// Run-scoped TTS handle. Kept on the main thread (the underlying `Tts`
/// wraps an `Rc` and is therefore `!Send`).
pub struct TtsEngine {
    inner: Tts,
    fallback_rate: f32,
    voice_rate_lang_code: Option<String>,
    voice_base_rate: Option<f32>,
}

impl TtsEngine {
    /// Build the platform default backend. Fails when no backend can be
    /// initialised (e.g. Linux without speech-dispatcher running).
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let inner = Tts::default()?;
        let fallback_rate = inner.get_rate().unwrap_or_else(|_| inner.normal_rate());
        Ok(Self {
            inner,
            fallback_rate,
            voice_rate_lang_code: None,
            voice_base_rate: None,
        })
    }

    pub fn runtime_support(&self) -> TtsRuntimeSupport {
        TtsRuntimeSupport::from_features(self.inner.supported_features())
    }

    pub fn speak_request(
        &mut self,
        request: TtsRequest<'_>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let support = self.runtime_support();
        let lang_code = request.lang.code();
        let lang_changed = self.voice_rate_lang_code.as_deref() != Some(lang_code);
        if support.can_set_rate && lang_changed {
            let _ = self.inner.set_rate(self.fallback_rate);
        }
        if let Some(voice) = pick_voice(&self.inner, request.lang) {
            // Voice selection is best-effort — a backend that doesn't
            // support `set_voice` (or rejects this voice) shouldn't kill
            // the run; we still want to attempt the speak call.
            let _ = self.inner.set_voice(&voice);
        }

        let profile = TtsProfile::for_request(request.kind);
        if support.can_set_rate {
            if lang_changed {
                self.voice_rate_lang_code = Some(lang_code.to_string());
                self.voice_base_rate = Some(self.inner.get_rate().unwrap_or(self.fallback_rate));
            }
            let base_rate = self.voice_base_rate.unwrap_or(self.fallback_rate);
            let rate = (base_rate * profile.rate_multiplier)
                .clamp(self.inner.min_rate(), self.inner.max_rate());
            let _ = self.inner.set_rate(rate);
        }

        self.inner.speak(request.text, profile.interrupt)?;
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
    use super::*;

    #[test]
    fn prompt_replay_profile_is_slightly_slower_than_first_read() {
        let prompt = TtsProfile::for_request(TtsRequestKind::PromptAnswer);
        let replay = TtsProfile::for_request(TtsRequestKind::PromptReplay);
        assert!(replay.rate_multiplier < prompt.rate_multiplier);
    }

    #[test]
    fn early_boss_hint_profile_is_slowest() {
        let early = TtsProfile::for_request(TtsRequestKind::BossHint { layer: 1 });
        let late = TtsProfile::for_request(TtsRequestKind::BossHint { layer: 5 });
        let reveal = TtsProfile::for_request(TtsRequestKind::BossReveal);
        assert!(early.rate_multiplier < late.rate_multiplier);
        assert!(late.rate_multiplier < reveal.rate_multiplier);
    }

    #[test]
    fn preferred_runtime_support_requires_stop_and_rate() {
        let support = TtsRuntimeSupport::from_features(Features {
            stop: true,
            rate: true,
            voice: true,
            ..Features::default()
        });
        assert_eq!(support.level, TtsSupportLevel::Preferred);
        assert!(support.can_choose_voice);
    }

    #[test]
    fn basic_runtime_support_is_still_usable() {
        let support = TtsRuntimeSupport::from_features(Features {
            stop: false,
            rate: false,
            voice: true,
            ..Features::default()
        });
        assert_eq!(support.level, TtsSupportLevel::Basic);
        assert!(support.can_choose_voice);
        assert!(!support.can_set_rate);
    }
}
