use clap::{Parser, ValueEnum};
use ifl_core::{IflCore, InputEvent};
use std::io::{self, Read};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Input text to analyze
    #[arg(short, long)]
    text: Option<String>,

    /// Simulation mode
    #[arg(short, long, value_enum, default_value_t = Mode::Typed)]
    mode: Mode,

    /// Typing speed in WPM (only for Typed mode)
    #[arg(long, default_value_t = 60)]
    wpm: u64,

    /// Replay events from file
    #[arg(long)]
    replay: Option<String>,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum Mode {
    Typed,
    Paste,
    Mixed,
}

fn main() {
    let args = Args::parse();
    let core = IflCore::new();

    if let Some(replay_file) = args.replay {
        let json = std::fs::read_to_string(replay_file).expect("Failed to read replay file");
        let id = core.import_events(&json).expect("Failed to import events");

        // For replay, we might not have the final text easily unless we reconstruct it or it's in the file.
        // But finalize_message needs text.
        // Let's assume for now we just want to see the profile based on events.
        // But wait, StructureAnalyzer needs text.
        // We can reconstruct text from events if we really want, but that's complex (handling backspaces etc).
        // For this simple CLI, let's just say "Replay analysis requires text reconstruction which is not yet implemented fully".
        // OR, we can just pass a dummy text if we only care about timing/source features.
        // Let's try to pass dummy text for now.

        match core.finalize_message(&id, "") {
            Ok(json) => println!("{}", json),
            Err(e) => eprintln!("Error: {}", e),
        }
        return;
    }

    // Get input text (arg or stdin)
    let text = match args.text {
        Some(t) => t,
        None => {
            let mut buffer = String::new();
            io::stdin().read_to_string(&mut buffer).unwrap();
            buffer
        }
    };

    if text.trim().is_empty() {
        eprintln!("Error: No input text provided.");
        return;
    }

    let core = IflCore::new();
    let id = core.start_message();
    let mut ts = 1000; // Start at 1s

    match args.mode {
        Mode::Typed => {
            // Simulate typing
            let char_delay_ms = (60_000.0 / (args.wpm as f64 * 5.0)) as u64;

            for ch in text.chars() {
                core.push_event(&id, InputEvent::KeyInsert { ch, ts })
                    .unwrap();
                ts += char_delay_ms;
            }
        }
        Mode::Paste => {
            // Simulate paste
            core.push_event(
                &id,
                InputEvent::Paste {
                    length: text.len(),
                    ts,
                },
            )
            .unwrap();
            ts += 100;
        }
        Mode::Mixed => {
            // Simulate mixed (half typed, half pasted)
            let split = text.len() / 2;
            let (first, second) = text.split_at(split);

            // Type first half
            let char_delay_ms = (60_000.0 / (args.wpm as f64 * 5.0)) as u64;
            for ch in first.chars() {
                core.push_event(&id, InputEvent::KeyInsert { ch, ts })
                    .unwrap();
                ts += char_delay_ms;
            }

            // Paste second half
            core.push_event(
                &id,
                InputEvent::Paste {
                    length: second.len(),
                    ts,
                },
            )
            .unwrap();
            ts += 500;
        }
    }

    // Submit
    core.push_event(&id, InputEvent::Submit { ts }).unwrap();

    // Finalize
    match core.finalize_message(&id, &text) {
        Ok(json) => println!("{}", json),
        Err(e) => eprintln!("Error: {}", e),
    }
}
