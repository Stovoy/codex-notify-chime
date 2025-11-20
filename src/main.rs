use rodio::cpal::BufferSize;
use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink, Source, StreamError};
use serde::Deserialize;
use std::error::Error;
use std::io::Cursor;
use std::path::PathBuf;
use std::{env, fs};

// Embedded MP3 audio so we do not depend on external files at runtime.
const NOTIFICATION_AUDIO: &[u8] = include_bytes!("../assets/notify.mp3");
const STREAM_BUFFER_FRAME_SIZES: [u32; 3] = [16_384, 8_192, 4_096];

#[derive(Deserialize)]
struct Notification {
    #[serde(rename = "type")]
    kind: String,
    #[serde(rename = "thread-id")]
    thread_id: Option<String>,
    #[serde(rename = "last-assistant-message")]
    last_assistant_message: Option<String>,
    #[serde(rename = "input-messages")]
    input_messages: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct AppConfig {
    volume: Option<f32>,
}

#[derive(Copy, Clone)]
struct PlaybackPreferences {
    volume: f32,
}

fn main() {
    std::process::exit(match run() {
        Ok(()) => 0,
        Err(err) => {
            eprintln!("{err}");
            1
        }
    });
}

fn run() -> Result<(), Box<dyn Error>> {
    let (notification_json, verbose) = parse_args()?;
    let playback_preferences = load_playback_preferences(verbose);

    let notification: Notification = serde_json::from_str(&notification_json)
        .map_err(|err| format!("Failed to parse notify payload as JSON: {err}"))?;

    play_sound_for_event(&notification, verbose, playback_preferences)?;
    Ok(())
}

fn parse_args() -> Result<(String, bool), String> {
    let mut args = std::env::args().skip(1).peekable();
    let mut verbose = false;

    if matches!(args.peek(), Some(flag) if flag.as_str() == "--verbose") {
        verbose = true;
        args.next();
    }

    let notification_json = args.next().ok_or_else(|| {
        String::from(
            "Usage: codex-notify-chime [--verbose] <NOTIFICATION_JSON>\n\
             Expected Codex to invoke this binary with a single JSON argument.",
        )
    })?;

    if args.next().is_some() {
        return Err(String::from(
            "Usage: codex-notify-chime [--verbose] <NOTIFICATION_JSON>\n\
             Expected Codex to invoke this binary with a single JSON argument.",
        ));
    }

    Ok((notification_json, verbose))
}

fn play_sound_for_event(
    notification: &Notification,
    verbose: bool,
    playback_preferences: PlaybackPreferences,
) -> Result<(), Box<dyn Error>> {
    let event_type = notification.kind.as_str();

    if verbose && event_type != "agent-turn-complete" {
        eprintln!("Codex notify event '{event_type}' is not recognized; using default sound");
    }

    if verbose {
        if let Some(last_message) = &notification.last_assistant_message {
            println!("Codex notify ({event_type}): {last_message}");
        } else if let Some(inputs) = &notification.input_messages {
            println!("Codex notify ({event_type}): {}", inputs.join(" "));
        } else if let Some(thread_id) = &notification.thread_id {
            println!("Codex notify ({event_type}) for thread {thread_id}");
        } else {
            println!("Codex notify ({event_type})");
        }
    }

    play_notification(verbose, playback_preferences)
}

fn play_notification(
    verbose: bool,
    playback_preferences: PlaybackPreferences,
) -> Result<(), Box<dyn Error>> {
    let mut stream = open_buffered_stream(verbose)?;
    stream.log_on_drop(false);

    let sink = Sink::connect_new(stream.mixer());
    let source = Decoder::new(Cursor::new(NOTIFICATION_AUDIO))?.buffered();

    sink.set_volume(playback_preferences.volume);
    sink.append(source);
    sink.sleep_until_end();
    Ok(())
}

fn open_buffered_stream(verbose: bool) -> Result<OutputStream, StreamError> {
    let mut attempted = false;

    for &frames in &STREAM_BUFFER_FRAME_SIZES {
        match OutputStreamBuilder::from_default_device()
            .map(|builder| builder.with_buffer_size(BufferSize::Fixed(frames)))
            .and_then(|builder| builder.open_stream())
        {
            Ok(stream) => return Ok(stream),
            Err(err) => {
                attempted = true;
                if verbose {
                    eprintln!("Audio stream with {frames} frame buffer rejected: {err}");
                }
            }
        }
    }

    if attempted && verbose {
        eprintln!("Falling back to default audio buffer size");
    }

    OutputStreamBuilder::open_default_stream()
}

fn load_playback_preferences(verbose: bool) -> PlaybackPreferences {
    let default = PlaybackPreferences { volume: 1.0 };
    let home = match env::var("HOME") {
        Ok(home) => PathBuf::from(home),
        Err(err) => {
            if verbose {
                eprintln!("HOME environment variable not set; using default volume ({err})");
            }
            return default;
        }
    };

    let config_path = home.join(".codex").join("notify.toml");
    let contents = match fs::read_to_string(&config_path) {
        Ok(contents) => contents,
        Err(err) => {
            if verbose {
                eprintln!("Unable to read config at {}: {err}", config_path.display());
            }
            return default;
        }
    };

    let parsed: AppConfig = match toml::from_str(&contents) {
        Ok(config) => config,
        Err(err) => {
            if verbose {
                eprintln!(
                    "Failed to parse TOML config at {}: {err}",
                    config_path.display()
                );
            }
            return default;
        }
    };

    match parsed.volume.filter(|volume| (0.0..=1.0).contains(volume)) {
        Some(volume) => PlaybackPreferences { volume },
        None => {
            if verbose {
                eprintln!(
                    "Volume missing or out of range in {} (expected 0.0-1.0); using full volume",
                    config_path.display()
                );
            }
            default
        }
    }
}
