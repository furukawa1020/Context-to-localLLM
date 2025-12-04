# IFL Core (Input Form Layer)

Rust implementation of the Input Form Layer.

## Features

- **Input Analysis**: Tracks typing speed, bursts, pauses, and editing behavior.
- **Structure Analysis**: Detects code blocks, bullet points, and Japanese text characteristics.
- **Rule Engine**: Generates "Answer Mode" tags (Summarize, Refine, etc.) based on input patterns.

## CLI Usage

You can use the CLI to test the analysis logic.

```bash
# Analyze a string (simulated typing)
cargo run -- --text "これはテストです。" --mode typed

# Analyze a pasted string
cargo run -- --text "Long pasted text..." --mode paste

# Analyze from stdin
echo "Hello world" | cargo run
```

## Testing

```bash
cargo test
```
