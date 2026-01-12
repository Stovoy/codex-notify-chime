# Codex Notify Chime

Codex Notify Chime is a tiny Rust utility that plays a cheerful alert whenever the OpenAI Codex agent sends a notification. It plugs into the Codex CLI so that, whenever the CLI produces an agent event, this binary prints a short summary to the terminal and plays a pleasant sound that ships with the project.

## Why You'll Like It
- Works out of the box: the notification sound is embedded in the binary—no extra assets to ship.
- Resilient audio output: automatically retries with smaller buffer sizes if the audio device rejects the default configuration.
- Human-friendly console output that surfaces the most interesting detail from the notification payload.

## Prerequisites
- Rust toolchain (edition 2024 or newer) installed via [rustup](https://rustup.rs/).
- To get this to work in WSL, install ALSA development headers (and pkg-config if missing):
  ```bash
  sudo apt update
  sudo apt install pkg-config libasound2-dev
  ```

## Install
```bash
cargo install --path .
```

Once installed, tell Codex CLI to invoke the notifier by adding it to `~/.codex/config.toml`:
```toml
notify = ["codex-notify-chime"]
```
Codex will now launch the notifier automatically for every agent notification, so there is no need to call the binary manually.

### Optional: Configure Volume
Create `~/.codex/notify.toml` to set playback volume between `0.0` (mute) and `1.0` (full volume). Example for 5% volume:

```toml
volume = 0.05
```

If the file is missing or the value is out of range, the chime plays at 100% volume by default.

## Build & Run
Play a quick test chime:
```bash
cargo run --release -- --test
```

Or pass a full notification payload:
```bash
cargo run --release -- \
  '{"type":"agent-turn-complete","last-assistant-message":"All tasks complete!"}'
```

Manual invocation like the above is only useful for local testing or development; when Codex runs it, the CLI passes the JSON payload for you. When invoked without an argument, the program prints usage instructions and exits with a non-zero status.

## Integrating With Codex CLI
Configure the Codex CLI to call this binary when agent notifications are emitted. Codex will pass the notification payload as a single JSON argument; Codex Notify Chime will log a short summary and play the embedded sound. Any unknown event types still trigger the default sound so you never miss an update.

## Development Notes
- The embedded MP3 lives in `assets/notify.mp3` and is included in the executable at compile time.
- Audio playback is handled through [rodio](https://crates.io/crates/rodio). If you encounter stream initialization errors, the program will automatically fall back to default audio settings and log what happened.

Happy hacking, and enjoy the chime!
