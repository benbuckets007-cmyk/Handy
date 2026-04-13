use anyhow::Result;
use reqwest::blocking::{multipart, Client};
use reqwest::StatusCode;
use serde::Deserialize;
use std::io::Cursor;
use std::thread;
use std::time::Duration;

pub const STT_CLOUD_PROVIDER_ID: &str = "cloud";
pub const STT_CLOUD_DEFAULT_SAMPLE_RATE_HZ: u32 = 16_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloudErrorClass {
    Timeout,
    Network,
    Auth,
    Http429,
    Http5xx,
    Http4xx,
    InvalidResponse,
    RequestConfig,
}

impl CloudErrorClass {
    pub fn is_retryable(self) -> bool {
        matches!(
            self,
            CloudErrorClass::Timeout
                | CloudErrorClass::Network
                | CloudErrorClass::Http429
                | CloudErrorClass::Http5xx
        )
    }

    pub fn as_str(self) -> &'static str {
        match self {
            CloudErrorClass::Timeout => "timeout",
            CloudErrorClass::Network => "network",
            CloudErrorClass::Auth => "auth",
            CloudErrorClass::Http429 => "http_429",
            CloudErrorClass::Http5xx => "http_5xx",
            CloudErrorClass::Http4xx => "http_4xx",
            CloudErrorClass::InvalidResponse => "invalid_response",
            CloudErrorClass::RequestConfig => "request_config",
        }
    }
}

#[derive(Debug)]
pub struct CloudTranscriptionError {
    pub class: CloudErrorClass,
    pub message: String,
}

impl CloudTranscriptionError {
    pub fn is_retryable(&self) -> bool {
        self.class.is_retryable()
    }
}

#[derive(Debug, Deserialize)]
struct TranscriptionResponse {
    text: Option<String>,
}

pub fn transcribe_cloud_wav_bytes(
    wav_bytes: Vec<u8>,
    base_url: &str,
    api_key: &str,
    model: &str,
    connect_timeout_ms: u64,
    request_timeout_ms: u64,
    allow_retry: bool,
) -> Result<String, CloudTranscriptionError> {
    let max_attempts = if allow_retry { 2 } else { 1 };
    let mut last_error: Option<CloudTranscriptionError> = None;

    for attempt in 1..=max_attempts {
        match transcribe_once(
            wav_bytes.clone(),
            base_url,
            api_key,
            model,
            connect_timeout_ms,
            request_timeout_ms,
        ) {
            Ok(text) => return Ok(text),
            Err(err) => {
                let should_retry = err.is_retryable() && attempt < max_attempts;
                last_error = Some(err);
                if should_retry {
                    thread::sleep(Duration::from_millis(300));
                    continue;
                }
                break;
            }
        }
    }

    Err(last_error.unwrap_or(CloudTranscriptionError {
        class: CloudErrorClass::Network,
        message: "Cloud transcription failed".to_string(),
    }))
}

pub fn wav_bytes_from_f32(
    audio: &[f32],
    sample_rate_hz: u32,
) -> Result<Vec<u8>, CloudTranscriptionError> {
    let mut cursor = Cursor::new(Vec::new());
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: sample_rate_hz,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer =
        hound::WavWriter::new(&mut cursor, spec).map_err(|e| CloudTranscriptionError {
            class: CloudErrorClass::RequestConfig,
            message: format!("Failed to initialize WAV encoder: {}", e),
        })?;

    for sample in audio {
        let clamped = sample.clamp(-1.0, 1.0);
        let pcm = (clamped * i16::MAX as f32) as i16;
        writer
            .write_sample(pcm)
            .map_err(|e| CloudTranscriptionError {
                class: CloudErrorClass::RequestConfig,
                message: format!("Failed to encode WAV audio: {}", e),
            })?;
    }

    writer.finalize().map_err(|e| CloudTranscriptionError {
        class: CloudErrorClass::RequestConfig,
        message: format!("Failed to finalize WAV audio: {}", e),
    })?;

    Ok(cursor.into_inner())
}

fn transcribe_once(
    wav_bytes: Vec<u8>,
    base_url: &str,
    api_key: &str,
    model: &str,
    connect_timeout_ms: u64,
    request_timeout_ms: u64,
) -> Result<String, CloudTranscriptionError> {
    let client = Client::builder()
        .connect_timeout(Duration::from_millis(connect_timeout_ms))
        .timeout(Duration::from_millis(request_timeout_ms))
        .build()
        .map_err(|e| CloudTranscriptionError {
            class: CloudErrorClass::RequestConfig,
            message: format!("Failed to build cloud transcription client: {}", e),
        })?;

    let file_part = multipart::Part::bytes(wav_bytes)
        .file_name("audio.wav")
        .mime_str("audio/wav")
        .map_err(|e| CloudTranscriptionError {
            class: CloudErrorClass::RequestConfig,
            message: format!("Failed to build cloud transcription payload: {}", e),
        })?;

    let form = multipart::Form::new()
        .text("model", model.to_string())
        .part("file", file_part);

    let response = client
        .post(format!(
            "{}/audio/transcriptions",
            base_url.trim_end_matches('/')
        ))
        .bearer_auth(api_key)
        .multipart(form)
        .send()
        .map_err(classify_reqwest_error)?;

    let status = response.status();
    if !status.is_success() {
        return Err(classify_http_status(status));
    }

    let payload: TranscriptionResponse = response.json().map_err(|e| CloudTranscriptionError {
        class: CloudErrorClass::InvalidResponse,
        message: format!("Cloud response could not be parsed: {}", e),
    })?;

    let text = payload.text.unwrap_or_default().trim().to_string();

    if text.is_empty() {
        return Err(CloudTranscriptionError {
            class: CloudErrorClass::InvalidResponse,
            message: "Cloud response was missing transcription text".to_string(),
        });
    }

    Ok(text)
}

fn classify_reqwest_error(err: reqwest::Error) -> CloudTranscriptionError {
    if err.is_timeout() {
        return CloudTranscriptionError {
            class: CloudErrorClass::Timeout,
            message: "Cloud transcription timed out".to_string(),
        };
    }

    if err.is_connect() || err.is_request() {
        return CloudTranscriptionError {
            class: CloudErrorClass::Network,
            message: format!("Cloud transcription transport error: {}", err),
        };
    }

    CloudTranscriptionError {
        class: CloudErrorClass::Network,
        message: format!("Cloud transcription request failed: {}", err),
    }
}

fn classify_http_status(status: StatusCode) -> CloudTranscriptionError {
    if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
        return CloudTranscriptionError {
            class: CloudErrorClass::Auth,
            message: "Cloud transcription authentication failed. Check your API key.".to_string(),
        };
    }

    if status == StatusCode::TOO_MANY_REQUESTS {
        return CloudTranscriptionError {
            class: CloudErrorClass::Http429,
            message: "Cloud transcription is rate-limited. Please try again.".to_string(),
        };
    }

    if status.is_server_error() {
        return CloudTranscriptionError {
            class: CloudErrorClass::Http5xx,
            message: format!("Cloud transcription server error ({})", status.as_u16()),
        };
    }

    CloudTranscriptionError {
        class: CloudErrorClass::Http4xx,
        message: format!("Cloud transcription failed with status {}", status.as_u16()),
    }
}
