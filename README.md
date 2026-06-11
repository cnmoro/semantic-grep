# semanticgrep

A semantic-aware grep tool — works like **ripgrep** but also lets you search by **cosine similarity** using [Model2Vec](https://github.com/MinishLab/model2vec-rs) static embeddings.

```
semanticgrep [OPTIONS] <PATTERN> [PATH...]
```

## Features

- **Regex search** — fully ripgrep-compatible CLI: `-i`, `-w`, `-F`, `-v`, `-n`, `-c`, `-l`, `-C`/`-A`/`-B` context, `-g` glob, `-H`/`-h` filenames, `--color`
- **Semantic search** — `-s 0.75` to find lines semantically similar to your query, not just exact regex matches
- **Fast** — uses `model2vec-rs` with static embeddings (8000 samples/sec on a single CPU core)
- **Gitignore-aware** — respects `.gitignore` automatically (via the `ignore` crate)
- **Cross-platform** — native builds for Linux (amd64 + arm64), macOS (x86 + arm), Windows x64

## Install

```bash
cargo install semanticgrep
```

Or download a pre-built binary from the [releases page](https://github.com/cnmoro/semantic-grep/releases).

## Usage

### Regex search (standard)

```bash
# Basic search
semanticgrep "fn main" src/

# Case-insensitive
semanticgrep -i "TODO" .

# Count matches
semanticgrep -c "error" log.txt

# Files with matches only
semanticgrep -l "password" .

# Context lines
semanticgrep -C 3 "unsafe" src/

# Glob filtering
semanticgrep -g "*.rs" "impl"

# File type filtering
semanticgrep -t rust "async"
```

### Semantic search

```bash
# Find lines semantically similar to "database connection error"
semanticgrep -s 0.75 "database connection error" src/

# Lower threshold = more matches, higher = stricter
semanticgrep -s 0.5 "memory leak" .
semanticgrep -s 0.9 "exact match only" file.txt

# Use a different model (default: minishlab/potion-base-8M)
semanticgrep -s 0.6 -m "minishlab/potion-base-32M" "config issue" .

# Combined with ripgrep flags
semanticgrep -s 0.7 -C 2 -g "*.py" "import error"
```

## How semantic search works

1. The query string is encoded into a 768-dimension vector using `model2vec-rs`
2. Every line in the target file is encoded into the same embedding space
3. Cosine similarity is computed between the query and each line
4. Lines with similarity ≥ the threshold are displayed, sorted by line number

## Flags

| Flag | Long | Description |
|------|------|-------------|
| `-s <F>` | `--semantic-threshold <F>` | Enable semantic search (threshold 0.0–1.0) |
| `-m <ID>` | `--model <ID>` | Model ID or local path (default: `minishlab/potion-base-8M`) |
| `-i` | `--ignore-case` | Case-insensitive regex search |
| `-F` | `--fixed-strings` | Treat pattern as literal string |
| `-w` | `--word-regexp` | Match only whole words |
| `-v` | `--invert-match` | Select non-matching lines |
| `-n` | `--line-number` | Show line numbers (default for stdout) |
| `-N` | `--no-line-number` | Suppress line numbers |
| `-c` | `--count` | Show match count per file |
| `-l` | `--files-with-matches` | Show only filenames |
| `-C <N>` | `--context <N>` | Show N lines around each match |
| `-A <N>` | `--after-context <N>` | Show N lines after each match |
| `-B <N>` | `--before-context <N>` | Show N lines before each match |
| `-g <G>` | `--glob <G>` | Include only files matching glob |
| `-t <T>` | `--type <T>` | Filter by file type |
| `-H` | `--with-filename` | Print filenames with results |
| `-h` | `--no-filename` | Suppress filenames |
| `--color <W>` | | When to use color: `auto`, `always`, `never` |
| `-j <N>` | `--threads <N>` | Thread count (reserved) |

## Development

```bash
cargo build
cargo test
cargo run -- --color never "pattern" path/
```

### Running semantic tests

Some integration tests download a ~8 MB model from Hugging Face Hub on first run (cached to `~/.cache/huggingface/hub/`).

## License

MIT
