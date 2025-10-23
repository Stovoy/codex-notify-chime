use rodio::cpal::BufferSize;
use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink, Source, StreamError};
use serde::Deserialize;
use std::error::Error;
use std::io::Cursor;

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

    let notification: Notification = serde_json::from_str(&notification_json)
        .map_err(|err| format!("Failed to parse notify payload as JSON: {err}"))?;

    play_sound_for_event(&notification, verbose)?;
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

    play_notification(verbose)
}

fn play_notification(verbose: bool) -> Result<(), Box<dyn Error>> {
    let mut stream = open_buffered_stream(verbose)?;
    stream.log_on_drop(false);

    let sink = Sink::connect_new(stream.mixer());
    let source = Decoder::new(Cursor::new(NOTIFICATION_AUDIO))?.buffered();

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
