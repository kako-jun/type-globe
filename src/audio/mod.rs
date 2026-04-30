//! Audio output. Currently only TTS (#28); future modules (sfx, music)
//! would live alongside `tts.rs` here.

pub mod tts;

pub use tts::TtsEngine;
