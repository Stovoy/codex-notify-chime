use rodio::{Decoder, OutputStream, Sink};
use serde::Deserialize;
use std::error::Error;
use std::io::Cursor;

// Embedded MP3 audio so we do not depend on external files at runtime.
const NOTIFICATION_AUDIO: &[u8] = include_bytes!("../assets/notify.mp3");

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
            "Usage: codex-notify <NOTIFICATION_JSON>\n\
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
    let (_stream, stream_handle) = OutputStream::try_default()?;
    let sink = Sink::try_new(&stream_handle)?;
    let source = Decoder::new(Cursor::new(NOTIFICATION_AUDIO))?;

    sink.append(source);
    sink.sleep_until_end();
    Ok(())
}
