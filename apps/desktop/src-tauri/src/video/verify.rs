//! Verification of the final video file with the pinned FFprobe.

use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tdm_core::tools::{Tool, Toolbox, VerifiedTool};
use tdm_core::video::Format;

use super::views::ChoiceView;

const MAX_OUTPUT: usize = 64 * 1024;
const TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct VerificationReport {
    pub size: u64,
    pub duration: Option<String>,
    pub video: Option<String>,
    pub audio: Option<String>,
    pub diagnostics: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(super) struct SelectedFormat {
    pub id: String,
    pub height: Option<u32>,
    pub fps: Option<f64>,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub has_video: bool,
    pub has_audio: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum VerificationError {
    ToolUnavailable(String),
    FileUnavailable(String),
    EmptyFile,
    ProbeFailed(String),
    InvalidOutput(String),
    MissingVideo,
    MissingAudio,
    InvalidDuration,
    InvalidVideoDimensions,
    QualityMismatch { expected: u32, actual: u32 },
}

impl std::fmt::Display for VerificationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ToolUnavailable(reason) => {
                write!(
                    f,
                    "FFprobe could not be verified or started: {reason}; the file was kept"
                )
            }
            Self::FileUnavailable(reason) => {
                write!(
                    f,
                    "the downloaded file could not be read: {reason}; the file was kept"
                )
            }
            Self::EmptyFile => {
                f.write_str("the downloaded file is empty; it was kept for diagnosis")
            }
            Self::ProbeFailed(reason) => {
                write!(
                    f,
                    "FFprobe could not read the file: {reason}; the file was kept"
                )
            }
            Self::InvalidOutput(reason) => {
                write!(
                    f,
                    "FFprobe returned invalid inspection data: {reason}; the file was kept"
                )
            }
            Self::MissingVideo => {
                f.write_str("the selected format requires video, but no video stream was found")
            }
            Self::MissingAudio => {
                f.write_str("the selected format requires audio, but no audio stream was found")
            }
            Self::InvalidDuration => {
                f.write_str("FFprobe reported a zero or invalid media duration")
            }
            Self::InvalidVideoDimensions => {
                f.write_str("the video stream has no usable resolution")
            }
            Self::QualityMismatch { expected, actual } => write!(
                f,
                "the selected format requires {expected}p, but the file is {actual}p"
            ),
        }
    }
}

#[derive(Debug, Deserialize)]
struct ProbeOutput {
    #[serde(default)]
    streams: Vec<ProbeStream>,
    format: Option<ProbeFormat>,
}

#[derive(Debug, Deserialize)]
struct ProbeStream {
    codec_type: Option<String>,
    codec_name: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    avg_frame_rate: Option<String>,
    r_frame_rate: Option<String>,
    duration: Option<String>,
    disposition: Option<ProbeDisposition>,
}

#[derive(Debug, Deserialize)]
struct ProbeDisposition {
    attached_pic: Option<u8>,
}

#[derive(Debug, Deserialize)]
struct ProbeFormat {
    duration: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RequiredStreams {
    video: bool,
    audio: bool,
    any_media: bool,
}

#[derive(Debug, Clone)]
struct ExpectedQuality<'a> {
    format_id: &'a str,
    height: Option<u32>,
    fps: Option<f64>,
    video_codec: Option<&'a str>,
    audio_codec: Option<&'a str>,
}

pub(super) async fn verify_file(
    tools_dir: PathBuf,
    file: PathBuf,
    choice: ChoiceView,
    formats: Vec<SelectedFormat>,
) -> Result<VerificationReport, VerificationError> {
    let toolbox = Toolbox::new(tools_dir);
    if !file.is_absolute() {
        return Err(VerificationError::FileUnavailable(
            "the final file path is not absolute".into(),
        ));
    }
    let ffprobe = toolbox
        .verified(Tool::Ffprobe)
        .await
        .map_err(|error| VerificationError::ToolUnavailable(error.to_string()))?;
    let inspected_file = file.clone();
    let output = tokio::task::spawn_blocking(move || inspect(&inspected_file, ffprobe))
        .await
        .map_err(|error| VerificationError::ToolUnavailable(error.to_string()))??;
    assess(&file, output, &choice, &formats)
}

struct ProcessOutput {
    success: bool,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

fn inspect(file: &Path, ffprobe: VerifiedTool) -> Result<ProcessOutput, VerificationError> {
    ensure_nonempty_file(file)?;
    run_probe(ffprobe.path(), file)
}

fn ensure_nonempty_file(file: &Path) -> Result<u64, VerificationError> {
    let opened = tdm_platform::fs::open_regular_nofollow(file, false)
        .map_err(|error| VerificationError::FileUnavailable(error.to_string()))?;
    let size = opened
        .metadata()
        .map_err(|error| VerificationError::FileUnavailable(error.to_string()))?
        .len();
    if size == 0 {
        return Err(VerificationError::EmptyFile);
    }
    Ok(size)
}

fn run_probe(program: &Path, file: &Path) -> Result<ProcessOutput, VerificationError> {
    let mut child = Command::new(program)
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration:stream=codec_type,codec_name,width,height,avg_frame_rate,r_frame_rate,duration,disposition",
            "-show_streams",
            "-show_format",
            "-of",
            "json",
            "-probesize",
            "50000000",
            "-analyzeduration",
            "10000000",
        ])
        .arg(file)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| VerificationError::ToolUnavailable(error.to_string()))?;

    let stdout = child.stdout.take().ok_or_else(|| {
        VerificationError::ToolUnavailable("FFprobe stdout was unavailable".into())
    })?;
    let stderr = child.stderr.take().ok_or_else(|| {
        VerificationError::ToolUnavailable("FFprobe stderr was unavailable".into())
    })?;
    let stdout_reader = thread::spawn(move || read_bounded(stdout));
    let stderr_reader = thread::spawn(move || read_bounded(stderr));
    let deadline = Instant::now() + TIMEOUT;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(100)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(VerificationError::ToolUnavailable(
                    "FFprobe exceeded the 30-second inspection limit".into(),
                ));
            }
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(VerificationError::ToolUnavailable(error.to_string()));
            }
        }
    };
    let stdout = stdout_reader
        .join()
        .map_err(|_| VerificationError::ToolUnavailable("FFprobe stdout reader stopped".into()))?
        .map_err(|error| VerificationError::ToolUnavailable(error.to_string()))?;
    let stderr = stderr_reader
        .join()
        .map_err(|_| VerificationError::ToolUnavailable("FFprobe stderr reader stopped".into()))?
        .map_err(|error| VerificationError::ToolUnavailable(error.to_string()))?;

    Ok(ProcessOutput {
        success: status.success(),
        stdout,
        stderr,
    })
}

fn read_bounded(mut reader: impl Read) -> io::Result<Vec<u8>> {
    let mut kept = Vec::new();
    let mut buffer = [0; 8192];
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        let remaining = MAX_OUTPUT.saturating_sub(kept.len());
        kept.extend_from_slice(&buffer[..count.min(remaining)]);
    }
    Ok(kept)
}

fn assess(
    file: &Path,
    output: ProcessOutput,
    choice: &ChoiceView,
    formats: &[SelectedFormat],
) -> Result<VerificationReport, VerificationError> {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    if !output.success {
        return Err(VerificationError::ProbeFailed(if stderr.is_empty() {
            "FFprobe exited with an error".into()
        } else {
            stderr
        }));
    }
    let diagnostics = String::from_utf8_lossy(&output.stdout).into_owned();
    let parsed: ProbeOutput = serde_json::from_slice(&output.stdout)
        .map_err(|error| VerificationError::InvalidOutput(error.to_string()))?;
    let size = std::fs::metadata(file)
        .map_err(|error| VerificationError::FileUnavailable(error.to_string()))?
        .len();
    if size == 0 {
        return Err(VerificationError::EmptyFile);
    }

    let expected = expected_quality(choice, formats);
    let required = required_streams(choice, formats);
    let video_stream = parsed.streams.iter().find(|stream| {
        stream.codec_type.as_deref() == Some("video")
            && stream
                .disposition
                .as_ref()
                .and_then(|disposition| disposition.attached_pic)
                != Some(1)
    });
    let audio_stream = parsed
        .streams
        .iter()
        .find(|stream| stream.codec_type.as_deref() == Some("audio"));

    if required.video && video_stream.is_none() {
        return Err(VerificationError::MissingVideo);
    }
    if required.audio && audio_stream.is_none() {
        return Err(VerificationError::MissingAudio);
    }
    if required.any_media && video_stream.is_none() && audio_stream.is_none() {
        return Err(VerificationError::MissingVideo);
    }

    let duration = parsed
        .format
        .as_ref()
        .and_then(|format| format.duration.as_deref())
        .and_then(parse_positive);
    let stream_duration = parsed
        .streams
        .iter()
        .filter_map(|stream| stream.duration.as_deref().and_then(parse_positive))
        .max_by(f64::total_cmp);
    let duration = duration.or(stream_duration);
    let has_duration = parsed.format.as_ref().is_some_and(|format| {
        format
            .duration
            .as_deref()
            .is_some_and(|value| value.parse::<f64>().is_ok())
    }) || parsed.streams.iter().any(|stream| {
        stream
            .duration
            .as_deref()
            .is_some_and(|value| value.parse::<f64>().is_ok())
    });
    if has_duration && duration.is_none() {
        return Err(VerificationError::InvalidDuration);
    }

    let mut warnings = Vec::new();
    let video = if let Some(stream) = video_stream {
        let (Some(width), Some(height)) = (stream.width, stream.height) else {
            return Err(VerificationError::InvalidVideoDimensions);
        };
        if width == 0 || height == 0 {
            return Err(VerificationError::InvalidVideoDimensions);
        }
        if let Some(expected_height) = expected.as_ref().and_then(|quality| quality.height)
            && height < expected_height
        {
            return Err(VerificationError::QualityMismatch {
                expected: expected_height,
                actual: height,
            });
        }
        if let Some(expected) = &expected {
            if let (Some(wanted), Some(actual)) = (expected.fps, frame_rate(stream))
                && (wanted - actual).abs() > 1.0
            {
                warnings.push(format!(
                    "frame rate differs from format {}: expected {wanted} fps, got {actual} fps; accepted",
                    expected.format_id
                ));
            }
            if let (Some(wanted), Some(actual)) =
                (expected.video_codec, stream.codec_name.as_deref())
                && !same_codec_family(wanted, actual)
            {
                warnings.push(format!(
                    "video codec differs from format {}: expected {wanted}, got {actual}; accepted",
                    expected.format_id
                ));
            }
        }
        Some(format!(
            "{width}x{height}, {} fps, codec {}",
            frame_rate(stream)
                .map(|rate| format!("{rate:.3}"))
                .unwrap_or_else(|| "unknown".into()),
            stream.codec_name.as_deref().unwrap_or("unknown")
        ))
    } else {
        None
    };
    let audio = audio_stream.map(|stream| {
        let codec = stream.codec_name.as_deref().unwrap_or("unknown");
        if let Some(expected) = &expected
            && let Some(wanted) = expected.audio_codec
            && !same_codec_family(wanted, codec)
        {
            warnings.push(format!(
                "audio codec differs from format {}: expected {wanted}, got {codec}; accepted",
                expected.format_id
            ));
        }
        format!("codec {codec}")
    });
    if let Some(expected) = &expected
        && expected.height.is_none()
        && video_stream.is_some()
    {
        warnings.push(format!(
            "format {} did not include expected resolution metadata; stream presence was verified but quality could not be compared",
            expected.format_id
        ));
    }

    Ok(VerificationReport {
        size,
        duration: duration.map(|value| format!("{value:.3} seconds")),
        video,
        audio,
        diagnostics,
        warnings,
    })
}

fn required_streams(choice: &ChoiceView, formats: &[SelectedFormat]) -> RequiredStreams {
    match choice {
        ChoiceView::AudioOnly => RequiredStreams {
            video: false,
            audio: true,
            any_media: false,
        },
        ChoiceView::Merge { .. } => RequiredStreams {
            video: true,
            audio: true,
            any_media: false,
        },
        ChoiceView::Format { id } => {
            let selected = formats.iter().find(|format| format.id == *id);
            let video = selected.is_some_and(|format| format.has_video || format.height.is_some());
            let audio = selected.is_some_and(|format| format.has_audio);
            RequiredStreams {
                video,
                audio,
                any_media: !video && !audio,
            }
        }
        ChoiceView::Best => RequiredStreams {
            video: false,
            audio: false,
            any_media: true,
        },
    }
}

fn expected_quality<'a>(
    choice: &'a ChoiceView,
    formats: &'a [SelectedFormat],
) -> Option<ExpectedQuality<'a>> {
    let (id, expected_video, expected_audio) = match choice {
        ChoiceView::Format { id } => (id.as_str(), Some(id.as_str()), Some(id.as_str())),
        ChoiceView::Merge { video, audio } => {
            (video.as_str(), Some(video.as_str()), Some(audio.as_str()))
        }
        ChoiceView::AudioOnly | ChoiceView::Best => return None,
    };
    let video_format = expected_video.and_then(|id| formats.iter().find(|format| format.id == id));
    let audio_format = expected_audio.and_then(|id| formats.iter().find(|format| format.id == id));
    Some(ExpectedQuality {
        format_id: id,
        height: video_format.and_then(|format| format.height),
        fps: video_format.and_then(|format| format.fps),
        video_codec: video_format.and_then(|format| format.video_codec.as_deref()),
        audio_codec: audio_format.and_then(|format| format.audio_codec.as_deref()),
    })
}

pub(super) fn selected_formats(choice: &ChoiceView, formats: &[Format]) -> Vec<SelectedFormat> {
    let ids = match choice {
        ChoiceView::Format { id } => vec![id.as_str()],
        ChoiceView::Merge { video, audio } => vec![video.as_str(), audio.as_str()],
        ChoiceView::AudioOnly | ChoiceView::Best => Vec::new(),
    };
    ids.into_iter()
        .filter_map(|id| formats.iter().find(|format| format.id == id))
        .map(|format| SelectedFormat {
            id: format.id.clone(),
            height: format.height,
            fps: format.fps,
            video_codec: format.vcodec.clone(),
            audio_codec: format.acodec.clone(),
            has_video: format.has_video(),
            has_audio: format.has_audio(),
        })
        .collect()
}

fn parse_positive(text: &str) -> Option<f64> {
    let value = text.parse::<f64>().ok()?;
    (value.is_finite() && value > 0.0).then_some(value)
}

fn frame_rate(stream: &ProbeStream) -> Option<f64> {
    [
        stream.avg_frame_rate.as_deref(),
        stream.r_frame_rate.as_deref(),
    ]
    .into_iter()
    .flatten()
    .find_map(|value| {
        let (numerator, denominator) = value.split_once('/')?;
        let rate = numerator.parse::<f64>().ok()? / denominator.parse::<f64>().ok()?;
        (rate.is_finite() && rate > 0.0).then_some(rate)
    })
}

fn same_codec_family(expected: &str, actual: &str) -> bool {
    fn family(codec: &str) -> &str {
        let lower = codec.to_ascii_lowercase();
        if lower.starts_with("avc1") || lower.starts_with("h264") {
            "h264"
        } else if lower.starts_with("vp09") || lower.starts_with("vp9") {
            "vp9"
        } else if lower.starts_with("vp08") || lower.starts_with("vp8") {
            "vp8"
        } else if lower.starts_with("av01") || lower.starts_with("av1") {
            "av1"
        } else if lower.starts_with("mp4a") || lower.starts_with("aac") {
            "aac"
        } else if lower.starts_with("opus") {
            "opus"
        } else if lower.starts_with("hvc1") || lower.starts_with("hev1") || lower == "hevc" {
            "hevc"
        } else {
            codec
        }
    }
    family(expected) == family(actual)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn format(id: &str, height: u32, vcodec: &str, acodec: &str) -> SelectedFormat {
        SelectedFormat {
            id: id.into(),
            height: Some(height),
            fps: Some(30.0),
            video_codec: (vcodec != "none").then(|| vcodec.into()),
            audio_codec: (acodec != "none").then(|| acodec.into()),
            has_video: vcodec != "none",
            has_audio: acodec != "none",
        }
    }

    fn file(bytes: &[u8]) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("media.bin");
        fs::write(&path, bytes).unwrap();
        (dir, path)
    }

    fn successful(streams: serde_json::Value) -> ProcessOutput {
        ProcessOutput {
            success: true,
            stdout: serde_json::to_vec(&serde_json::json!({
                "format": { "duration": "12.5" },
                "streams": streams
            }))
            .unwrap(),
            stderr: Vec::new(),
        }
    }

    fn video(height: u32, codec: &str) -> serde_json::Value {
        serde_json::json!({
            "codec_type": "video",
            "codec_name": codec,
            "width": 1280,
            "height": height,
            "avg_frame_rate": "30000/1001",
            "duration": "12.5"
        })
    }

    fn audio(codec: &str) -> serde_json::Value {
        serde_json::json!({
            "codec_type": "audio",
            "codec_name": codec,
            "duration": "12.5"
        })
    }

    #[test]
    fn verifies_video_and_audio_streams_and_accepts_small_frame_rate_difference() {
        let (_dir, path) = file(b"local media fixture");
        let report = assess(
            &path,
            successful(serde_json::json!([video(720, "h264"), audio("aac")])),
            &ChoiceView::Merge {
                video: "137".into(),
                audio: "140".into(),
            },
            &[
                format("137", 720, "avc1.640028", "none"),
                format("140", 0, "none", "mp4a.40.2"),
            ],
        )
        .unwrap();
        assert!(
            report
                .video
                .as_deref()
                .is_some_and(|value| value.contains("1280x720"))
        );
        assert!(
            report
                .audio
                .as_deref()
                .is_some_and(|value| value.contains("aac"))
        );
        assert_eq!(report.duration.as_deref(), Some("12.500 seconds"));
        assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    }

    #[test]
    fn allows_a_silent_video_only_format() {
        let (_dir, path) = file(b"silent local video");
        let report = assess(
            &path,
            successful(serde_json::json!([video(1080, "h264")])),
            &ChoiceView::Format { id: "137".into() },
            &[format("137", 1080, "avc1.640028", "none")],
        )
        .unwrap();
        assert!(report.video.is_some());
        assert!(report.audio.is_none());
    }

    #[test]
    fn allows_audio_only_without_a_video_stream() {
        let (_dir, path) = file(b"local audio");
        let report = assess(
            &path,
            successful(serde_json::json!([audio("aac")])),
            &ChoiceView::AudioOnly,
            &[],
        )
        .unwrap();
        assert!(report.video.is_none());
        assert!(report.audio.is_some());
    }

    #[test]
    fn requires_both_streams_for_a_merged_choice() {
        let (_dir, path) = file(b"video only");
        assert_eq!(
            assess(
                &path,
                successful(serde_json::json!([video(720, "h264")])),
                &ChoiceView::Merge {
                    video: "137".into(),
                    audio: "140".into()
                },
                &[],
            ),
            Err(VerificationError::MissingAudio)
        );
    }

    #[test]
    fn rejects_empty_and_unreadable_media_without_deleting_it() {
        let (_dir, empty) = file(b"");
        assert_eq!(
            ensure_nonempty_file(&empty),
            Err(VerificationError::EmptyFile)
        );
        let (_dir, corrupt) = file(b"not media");
        let result = assess(
            &corrupt,
            ProcessOutput {
                success: false,
                stdout: Vec::new(),
                stderr: b"Invalid data found when processing input".to_vec(),
            },
            &ChoiceView::AudioOnly,
            &[],
        );
        assert!(matches!(result, Err(VerificationError::ProbeFailed(_))));
        assert_eq!(fs::read(corrupt).unwrap(), b"not media");
    }

    #[test]
    fn rejects_empty_and_invalid_duration_and_video_dimensions() {
        let (_dir, path) = file(b"some bytes");
        let mut output = successful(serde_json::json!([audio("aac")]));
        output.stdout =
            br#"{"format":{"duration":"0"},"streams":[{"codec_type":"audio","codec_name":"aac"}]}"#
                .to_vec();
        assert_eq!(
            assess(&path, output, &ChoiceView::AudioOnly, &[]),
            Err(VerificationError::InvalidDuration)
        );
        assert_eq!(
            assess(
                &path,
                successful(serde_json::json!([{
                    "codec_type": "video", "codec_name": "h264", "width": 0, "height": 0
                }])),
                &ChoiceView::Format { id: "137".into() },
                &[],
            ),
            Err(VerificationError::InvalidVideoDimensions)
        );
    }

    #[test]
    fn rejects_a_resolution_lower_than_the_selected_format() {
        let (_dir, path) = file(b"local media fixture");
        assert_eq!(
            assess(
                &path,
                successful(serde_json::json!([video(480, "h264")])),
                &ChoiceView::Format { id: "137".into() },
                &[format("137", 720, "avc1.640028", "none")],
            ),
            Err(VerificationError::QualityMismatch {
                expected: 720,
                actual: 480,
            })
        );
    }

    #[test]
    fn distinguishes_a_missing_ffprobe_from_a_bad_media_file() {
        let (_dir, path) = file(b"local media fixture");
        let result = run_probe(Path::new("missing-tdm-ffprobe"), &path);
        assert!(matches!(result, Err(VerificationError::ToolUnavailable(_))));
        assert_eq!(fs::read(path).unwrap(), b"local media fixture");
    }
}
