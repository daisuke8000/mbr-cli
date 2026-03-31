# mbr-cli

Rust-based async CLI/TUI tool for interacting with Metabase APIs.

## Features

- **Flat Command Structure**: Intuitive commands like `queries`, `run`, `collections`, `databases`, `tables`, `status`
- **AI-Agent Friendly**: Global `-j` flag for JSON output, structured error codes, clean stdout/stderr separation
- **Session Authentication**: Login with username/password, session tokens stored securely on disk
- **Rich TUI Experience**: Interactive terminal UI with keyboard navigation, search, sort, and filter
- **Multiple Output Formats**: Table (default), JSON (`-j`), and CSV (`--format csv`)
- **Structured Error Handling**: Machine-readable error codes and granular exit codes

## Quick Start

### 1. Configure Metabase URL

```bash
mbr-cli config set-url "https://your-metabase.com"
```

### 2. Login

```bash
mbr-cli login
```

Or use environment variables for non-interactive login:

```bash
export MBR_USERNAME="user@example.com"
export MBR_PASSWORD="your_password"
mbr-cli login
```

### 3. Start using

```bash
# List saved questions
mbr-cli queries

# Execute a question
mbr-cli run 123

# Get JSON output (pipe to jq, scripts, AI agents)
mbr-cli run 123 -j

# Launch interactive TUI
cargo run -p mbr-tui
```

## CLI Commands

Most commands have short aliases: `queries` → `q`, `collections` → `c`, `databases` → `db`, `config` → `cfg`.

### Queries

```bash
mbr-cli queries                         # List all saved questions
mbr-cli queries -s "sales"              # Search by name
mbr-cli queries -s "sales" -l 10        # Search with limit
mbr-cli queries --collection 5          # Filter by collection ID
mbr-cli queries -j                      # Output as JSON
```

### Run (Execute Questions)

```bash
mbr-cli run 123                         # Execute question by ID
mbr-cli run 123 -j                      # Output as JSON
mbr-cli run 123 --format csv            # Output as CSV
mbr-cli run 123 --param date=2024-01-01 # Execute with parameters
mbr-cli run 123 --full                  # Show all results
mbr-cli run 123 --offset 10 --limit 50  # Pagination: skip 10, show 50
mbr-cli run 123 --no-fullscreen         # Disable interactive mode
```

### Collections

```bash
mbr-cli collections                     # List all collections
mbr-cli collections -j                  # Output as JSON
mbr-cli collections --format csv        # Output as CSV
```

### Databases

```bash
mbr-cli databases                       # List all databases
mbr-cli databases -j                    # Output as JSON
mbr-cli databases --format csv          # Output as CSV
```

### Tables

```bash
mbr-cli tables 1                        # List tables (schema defaults to "public")
mbr-cli tables 1 public                 # List tables in database 1, schema "public"
mbr-cli tables 1 public -j              # Output as JSON
```

### Status & Configuration

```bash
mbr-cli status                          # Show connection status and config
mbr-cli status -j                       # Output as JSON
mbr-cli config set-url "https://..."    # Set Metabase server URL
mbr-cli config validate                 # Validate session and connection
mbr-cli config validate -j              # Validate with JSON output
```

### Authentication

```bash
mbr-cli login                           # Interactive login
mbr-cli login -j                        # Login with JSON output
mbr-cli logout                          # Logout and clear session
```

### Global Flags

| Flag | Short | Description |
|------|-------|-------------|
| `--json` | `-j` | Output as JSON (all commands) |
| `--verbose` | `-v` | Enable debug output |
| `--color` | | Color control: `auto`, `always`, `never` |
| `--config-dir` | | Custom configuration directory |

### Per-Command Flags

| Flag | Short | Description | Available on |
|------|-------|-------------|--------------|
| `--format` | `-f` | Output format: `table`, `json`, `csv` | Most commands |
| `--limit` | `-l` | Max results to return | `queries`, `run` |
| `--page-size` | | Rows per page in interactive mode (default: 20) | `run` |
| `--full` | | Show all results without limit | `run` |
| `--no-fullscreen` | | Disable interactive fullscreen mode | `run` |
| `--offset` | | Skip first N rows | `run` |
| `--param` | `-p` | Query parameter (key=value, repeatable) | `run` |

## AI Agent Integration

mbr-cli is designed for seamless use by AI agents (Claude Code, etc.) via subprocess:

```bash
# All commands support -j for machine-readable JSON
mbr-cli queries -j 2>/dev/null | jq '.[] | .name'

# Structured error output
mbr-cli run 999 -j
# -> {"error":{"code":"QUESTION_NOT_FOUND","message":"...","hint":"..."}}

# Granular exit codes for programmatic error handling
mbr-cli run 123 -j; echo $?
# 0 = success, 1 = CLI error, 2 = API error, 3 = auth error, 4 = config error

# Status messages go to stderr, data to stdout -- safe for piping
mbr-cli run 123 -j 2>/dev/null | jq .
```

## TUI Controls

### Global

| Key | Action |
|-----|--------|
| `q` | Quit application |
| `Esc` | Quit / Back from result |
| `1` / `2` / `3` | Switch to Questions / Collections / Databases |
| `Tab` | Next tab |
| `Shift+Tab` | Previous tab |
| `r` | Refresh data |
| `?` | Toggle help |

### Navigation

| Key | Action |
|-----|--------|
| `Up` / `k` | Move up |
| `Down` / `j` | Move down |
| `Left` / `h` | Scroll left (columns) |
| `Right` / `l` | Scroll right (columns) |
| `PgUp` / `PgDn` | Page up / down |
| `n` / `p` | Next / Previous page |
| `Home` / `g` | First page / First item |
| `End` / `G` | Last page / Last item |
| `Enter` | Execute query / Record detail |
| `/` | Search |
| `s` | Sort (result view) |
| `f` / `F` | Filter / Clear filter |
| `c` | Copy record(s) |
| `Space` | Toggle row selection |
| `Shift+Up/Down` | Range selection |
| `Ctrl+A` | Select all rows |

## Environment Variables

| Variable | Description |
|----------|-------------|
| `MBR_USERNAME` | Metabase username (for non-interactive login) |
| `MBR_PASSWORD` | Metabase password (for non-interactive login) |
| `MBR_URL` | Metabase server URL (alternative to config file) |

## Development

```bash
# Build all crates
cargo build

# Run tests
cargo test

# Run CLI
cargo run -p mbr-cli -- --help

# Run TUI
cargo run -p mbr-tui

# Full local check (format + lint + test)
make check

# CI-style check (non-modifying)
make check-ci
```

## License

Licensed under the MIT License. See [LICENSE](LICENSE) file for details.
