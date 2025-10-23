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
    let notification_json = std::env::args().nth(1).ok_or_else(|| {
        String::from(
            "Usage: codex-notify-chime <NOTIFICATION_JSON>\n\
             Expected Codex to invoke this binary with a single JSON argument.",
        )
    })?;

    let notification: Notification = serde_json::from_str(&notification_json)
        .map_err(|err| format!("Failed to parse notify payload as JSON: {err}"))?;

    play_sound_for_event(&notification)?;
    Ok(())
}

fn play_sound_for_event(notification: &Notification) -> Result<(), Box<dyn Error>> {
    let event_type = notification.kind.as_str();

    if event_type != "agent-turn-complete" {
        eprintln!("Codex notify event '{event_type}' is not recognized; using default sound");
    }

    if let Some(last_message) = &notification.last_assistant_message {
        println!("Codex notify ({event_type}): {last_message}");
    } else if let Some(inputs) = &notification.input_messages {
        println!("Codex notify ({event_type}): {}", inputs.join(" "));
    } else if let Some(thread_id) = &notification.thread_id {
        println!("Codex notify ({event_type}) for thread {thread_id}");
    } else {
        println!("Codex notify ({event_type})");
    }

    play_notification()
}

fn play_notification() -> Result<(), Box<dyn Error>> {
    let mut stream = open_buffered_stream()?;
    stream.log_on_drop(false);

    let sink = Sink::connect_new(stream.mixer());
    let source = Decoder::new(Cursor::new(NOTIFICATION_AUDIO))?.buffered();

    sink.append(source);
    sink.sleep_until_end();
    Ok(())
}

fn open_buffered_stream() -> Result<OutputStream, StreamError> {
    let mut attempted = false;

    for &frames in &STREAM_BUFFER_FRAME_SIZES {
        match OutputStreamBuilder::from_default_device()
            .map(|builder| builder.with_buffer_size(BufferSize::Fixed(frames)))
            .and_then(|builder| builder.open_stream())
        {
            Ok(stream) => return Ok(stream),
            Err(err) => {
                attempted = true;
                eprintln!("Audio stream with {frames} frame buffer rejected: {err}");
            }
        }
    }

    if attempted {
        eprintln!("Falling back to default audio buffer size");
    }

    OutputStreamBuilder::open_default_stream()
}
