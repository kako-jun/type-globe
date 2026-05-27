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
            level: SpeechSupportLevel::Preferred,
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

        let byte_len = response.byte_len.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "local speech command returned ok=true without byte_len",
            )
        })?;
        let mut wav_bytes = vec![0_u8; byte_len];
        self.stdout.read_exact(&mut wav_bytes)?;

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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn boss_hint_kind_maps_to_flat_protocol_parts() {
        assert_eq!(
            SpeechRequestKind::BossHint { layer: 3 }.protocol_parts(),
            ("boss_hint", Some(3))
        );
    }
}
