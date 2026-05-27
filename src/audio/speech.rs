//! Pluggable speech backend boundary for listening/RPG audio.
//!
//! `type-globe` keeps the OS TTS path as the default, but higher layers
//! should speak through this module so the same request shape can later
//! target `offline-voice-runtime` and sibling apps.

use crate::audio::tts::{TtsEngine, TtsRequest, TtsRequestKind};
use crate::types::Language;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};
use serde::Serialize;
use std::io::{self, BufRead, BufReader, Cursor, Read, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

const MAX_LOCAL_SPEECH_WAV_BYTES: usize = 32 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpeechRequestKind {
    PromptAnswer,
    PromptReplay,
    BossHint { layer: u8 },
    BossReveal,
}

impl From<SpeechRequestKind> for TtsRequestKind {
    fn from(kind: SpeechRequestKind) -> Self {
        match kind {
            SpeechRequestKind::PromptAnswer => Self::PromptAnswer,
            SpeechRequestKind::PromptReplay => Self::PromptReplay,
            SpeechRequestKind::BossHint { layer } => Self::BossHint { layer },
            SpeechRequestKind::BossReveal => Self::BossReveal,
        }
    }
}

impl SpeechRequestKind {
    fn protocol_parts(self) -> (&'static str, Option<u8>) {
        match self {
            SpeechRequestKind::PromptAnswer => ("prompt_answer", None),
            SpeechRequestKind::PromptReplay => ("prompt_replay", None),
            SpeechRequestKind::BossHint { layer } => ("boss_hint", Some(layer)),
            SpeechRequestKind::BossReveal => ("boss_reveal", None),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SpeechRequest<'a> {
    pub text: &'a str,
    pub lang: &'a Language,
    pub kind: SpeechRequestKind,
    /// Future-proof field for app-specific roles such as `kako_narrator`.
    pub voice_role: Option<&'a str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpeechSupportLevel {
    Preferred,
    Basic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpeechCapabilities {
    pub level: SpeechSupportLevel,
    pub can_stop: bool,
    pub can_set_rate: bool,
    pub can_choose_voice: bool,
    pub backend_name: &'static str,
}

pub struct SpeechBackendHandle {
    inner: SpeechBackendImpl,
}

enum SpeechBackendImpl {
    System(SystemSpeechBackend),
    LocalCommand(LocalCommandSpeechBackend),
}

impl SpeechBackendHandle {
    pub fn system() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            inner: SpeechBackendImpl::System(SystemSpeechBackend::new()?),
        })
    }

    pub fn local_command(command: String) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            inner: SpeechBackendImpl::LocalCommand(LocalCommandSpeechBackend::spawn(command)?),
        })
    }

    pub fn speak(&mut self, request: SpeechRequest<'_>) -> Result<(), Box<dyn std::error::Error>> {
        match &mut self.inner {
            SpeechBackendImpl::System(backend) => backend.speak(request),
            SpeechBackendImpl::LocalCommand(backend) => backend.speak(request),
        }
    }

    pub fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        match &mut self.inner {
            SpeechBackendImpl::System(backend) => backend.stop(),
            SpeechBackendImpl::LocalCommand(backend) => backend.stop(),
        }
    }

    #[allow(dead_code)]
    pub fn is_speaking(&self) -> bool {
        match &self.inner {
            SpeechBackendImpl::System(backend) => backend.is_speaking(),
            SpeechBackendImpl::LocalCommand(backend) => backend.is_speaking(),
        }
    }

    #[allow(dead_code)]
    pub fn capabilities(&self) -> SpeechCapabilities {
        match &self.inner {
            SpeechBackendImpl::System(backend) => backend.capabilities(),
            SpeechBackendImpl::LocalCommand(backend) => backend.capabilities(),
        }
    }
}

struct SystemSpeechBackend {
    tts: TtsEngine,
}

impl SystemSpeechBackend {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            tts: TtsEngine::new()?,
        })
    }

    fn speak(&mut self, request: SpeechRequest<'_>) -> Result<(), Box<dyn std::error::Error>> {
        self.tts.speak_request(TtsRequest {
            text: request.text,
            lang: request.lang,
            kind: request.kind.into(),
        })
    }

    fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.tts.stop()
    }

    fn is_speaking(&self) -> bool {
        self.tts.is_speaking()
    }

    fn capabilities(&self) -> SpeechCapabilities {
        let support = self.tts.runtime_support();
        SpeechCapabilities {
            level: match support.level {
                crate::audio::tts::TtsSupportLevel::Preferred => SpeechSupportLevel::Preferred,
                crate::audio::tts::TtsSupportLevel::Basic => SpeechSupportLevel::Basic,
            },
            can_stop: support.can_stop,
            can_set_rate: support.can_set_rate,
            can_choose_voice: support.can_choose_voice,
            backend_name: "system",
        }
    }
}

struct LocalCommandSpeechBackend {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    // `OutputStream` must stay alive for every Sink created from its handle.
    #[allow(dead_code)]
    output_stream: OutputStream,
    output_handle: OutputStreamHandle,
    sink: Option<Sink>,
}

impl LocalCommandSpeechBackend {
    fn spawn(command: String) -> Result<Self, Box<dyn std::error::Error>> {
        let mut child = Command::new("sh")
            .arg("-c")
            .arg(command)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;
        let stdin = child.stdin.take().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::BrokenPipe,
                "local speech command did not expose stdin",
            )
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::BrokenPipe,
                "local speech command did not expose stdout",
            )
        })?;
        let (output_stream, output_handle) = OutputStream::try_default()?;
        Ok(Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
            output_stream,
            output_handle,
            sink: None,
        })
    }

    fn speak(&mut self, request: SpeechRequest<'_>) -> Result<(), Box<dyn std::error::Error>> {
        self.stop()?;

        let profile = crate::audio::tts::TtsProfile::for_request(request.kind.into());
        let (kind, layer) = request.kind.protocol_parts();
        self.write_speak_command(&LocalSpeechCommand::Speak {
            text: request.text,
            lang: request.lang.code(),
            kind,
            layer,
            voice_role: request.voice_role,
            interrupt: profile.interrupt,
            rate_multiplier: profile.rate_multiplier,
        })
    }

    fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(sink) = self.sink.take() {
            sink.stop();
        }
        Ok(())
    }

    fn is_speaking(&self) -> bool {
        self.sink
            .as_ref()
            .map(|sink| !sink.empty())
            .unwrap_or(false)
    }

    fn capabilities(&self) -> SpeechCapabilities {
        SpeechCapabilities {
            level: SpeechSupportLevel::Basic,
            can_stop: true,
            can_set_rate: true,
            can_choose_voice: true,
            backend_name: "local-command",
        }
    }

    fn write_speak_command(
        &mut self,
        command: &LocalSpeechCommand<'_>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        serde_json::to_writer(&mut self.stdin, command)?;
        self.stdin.write_all(b"\n")?;
        self.stdin.flush()?;

        let response = self.read_response_header()?;
        if !response.ok {
            return Err(io::Error::other(
                response
                    .error
                    .unwrap_or_else(|| "local speech command returned ok=false".to_string()),
            )
            .into());
        }

        let byte_len = validate_local_wav_byte_len(&response)?;
        let wav_bytes = read_exact_local_wav_bytes(&mut self.stdout, byte_len)?;

        let source = Decoder::new(BufReader::new(Cursor::new(wav_bytes)))?;
        let sink = Sink::try_new(&self.output_handle)?;
        sink.append(source);
        self.sink = Some(sink);
        Ok(())
    }

    fn read_response_header(&mut self) -> Result<LocalSpeechResponse, Box<dyn std::error::Error>> {
        let mut line = String::new();
        let bytes = self.stdout.read_line(&mut line)?;
        if bytes == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "local speech command exited before returning audio header",
            )
            .into());
        }
        Ok(serde_json::from_str(line.trim())?)
    }
}

impl Drop for LocalCommandSpeechBackend {
    fn drop(&mut self) {
        let _ = serde_json::to_writer(&mut self.stdin, &LocalSpeechCommand::Shutdown);
        let _ = self.stdin.write_all(b"\n");
        let _ = self.stdin.flush();
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum LocalSpeechCommand<'a> {
    Speak {
        text: &'a str,
        lang: &'a str,
        kind: &'a str,
        layer: Option<u8>,
        voice_role: Option<&'a str>,
        interrupt: bool,
        rate_multiplier: f32,
    },
    Shutdown,
}

#[derive(serde::Deserialize)]
struct LocalSpeechResponse {
    ok: bool,
    #[serde(default)]
    byte_len: Option<usize>,
    #[serde(default)]
    error: Option<String>,
}

fn validate_local_wav_byte_len(response: &LocalSpeechResponse) -> io::Result<usize> {
    let byte_len = response.byte_len.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "local speech command returned ok=true without byte_len",
        )
    })?;
    if byte_len > MAX_LOCAL_SPEECH_WAV_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "local speech command returned oversized WAV payload: {byte_len} bytes (max {MAX_LOCAL_SPEECH_WAV_BYTES})"
            ),
        ));
    }
    Ok(byte_len)
}

fn read_exact_local_wav_bytes<R: Read>(reader: &mut R, byte_len: usize) -> io::Result<Vec<u8>> {
    let mut wav_bytes = vec![0_u8; byte_len];
    reader.read_exact(&mut wav_bytes)?;
    Ok(wav_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rodio::Source;
    use std::io::Cursor;

    #[test]
    fn local_speak_command_serializes_shared_request_shape() {
        let command = LocalSpeechCommand::Speak {
            text: "apple",
            lang: "en",
            kind: "prompt_replay",
            layer: None,
            voice_role: Some("neutral"),
            interrupt: true,
            rate_multiplier: 0.96,
        };
        let json = serde_json::to_string(&command).unwrap();
        assert!(json.contains(r#""type":"speak""#));
        assert!(json.contains(r#""text":"apple""#));
        assert!(json.contains(r#""lang":"en""#));
        assert!(json.contains(r#""kind":"prompt_replay""#));
        assert!(json.contains(r#""layer":null"#));
        assert!(json.contains(r#""voice_role":"neutral""#));
        assert!(json.contains(r#""rate_multiplier":0.96"#));
    }

    #[test]
    fn local_response_header_carries_audio_byte_len() {
        let header: LocalSpeechResponse =
            serde_json::from_str(r#"{"ok":true,"byte_len":42}"#).unwrap();
        assert!(header.ok);
        assert_eq!(header.byte_len, Some(42));
        assert!(header.error.is_none());
    }

    #[test]
    fn local_response_header_requires_byte_len_when_ok() {
        let header: LocalSpeechResponse = serde_json::from_str(r#"{"ok":true}"#).unwrap();
        let err = validate_local_wav_byte_len(&header).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn local_response_header_rejects_oversized_audio() {
        let header = LocalSpeechResponse {
            ok: true,
            byte_len: Some(MAX_LOCAL_SPEECH_WAV_BYTES + 1),
            error: None,
        };
        let err = validate_local_wav_byte_len(&header).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn local_wav_body_reader_rejects_truncated_body() {
        let mut bytes = Cursor::new(vec![0_u8; 3]);
        let err = read_exact_local_wav_bytes(&mut bytes, 4).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::UnexpectedEof);
    }

    #[test]
    fn minimal_wav_decodes_with_enabled_rodio_feature() {
        let wav = minimal_pcm_wav_bytes();
        let decoder = Decoder::new(BufReader::new(Cursor::new(wav))).unwrap();
        assert_eq!(decoder.channels(), 1);
        assert_eq!(decoder.sample_rate(), 8_000);
    }

    #[test]
    fn boss_hint_kind_maps_to_flat_protocol_parts() {
        assert_eq!(
            SpeechRequestKind::BossHint { layer: 3 }.protocol_parts(),
            ("boss_hint", Some(3))
        );
    }

    fn minimal_pcm_wav_bytes() -> Vec<u8> {
        let mut wav = Vec::new();
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&38_u32.to_le_bytes());
        wav.extend_from_slice(b"WAVE");
        wav.extend_from_slice(b"fmt ");
        wav.extend_from_slice(&16_u32.to_le_bytes());
        wav.extend_from_slice(&1_u16.to_le_bytes());
        wav.extend_from_slice(&1_u16.to_le_bytes());
        wav.extend_from_slice(&8_000_u32.to_le_bytes());
        wav.extend_from_slice(&16_000_u32.to_le_bytes());
        wav.extend_from_slice(&2_u16.to_le_bytes());
        wav.extend_from_slice(&16_u16.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&2_u32.to_le_bytes());
        wav.extend_from_slice(&0_i16.to_le_bytes());
        wav
    }
}
