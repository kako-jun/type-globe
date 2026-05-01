//! Audio output. Hosts the speech-dispatcher TTS wrapper (#28) and the
//! synthesized sound-effect cues used by Quiz mode (#73).

pub mod cues;
pub mod tts;

pub use cues::{Cue, CueEngine};
pub use tts::TtsEngine;
