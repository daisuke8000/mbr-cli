# mbr-cli

Rust-based async CLI/TUI tool for interacting with Metabase APIs.

## Features

- **Session Authentication**: Login with username/password, session tokens stored securely on disk
- **CLI Interface**: Command-line tool for scripting and quick operations
- **Rich TUI Experience**: Interactive terminal UI with keyboard navigation, search, sort, and filter
- **Async Operations**: Built on tokio for efficient I/O
- **Hierarchical Error Handling**: Comprehensive error system with troubleshooting hints

## Quick Start

### 1. Configure your Metabase URL

```bash
mbr-cli config set --url "https://your-metabase.com"
```

### 2. Login

```bash
mbr-cli login
```

You will be prompted for your username and password interactively. Alternatively, set environment variables for non-interactive login:

```bash
export MBR_USERNAME="user@example.com"
export MBR_PASSWORD="your_password"
mbr-cli login
```

### 3. Start using

```bash
# List available questions
mbr-cli query --list

# Execute a question
mbr-cli query 123

# Launch interactive TUI
cargo run -p mbr-tui
```

## CLI Commands

### Login / Logout

```bash
mbr-cli login                         # Login to Metabase (interactive prompt)
mbr-cli logout                        # Logout and clear session
```

### Configuration

```bash
mbr-cli config show                              # Display current configuration
mbr-cli config set --url "https://metabase.com"  # Set Metabase URL
mbr-cli config validate                          # Validate session and connection
```

### Query

```bash
mbr-cli query --list                        # List all questions
mbr-cli query --list --search "sales"       # Search questions by name
mbr-cli query --list --collection 5         # Filter by collection ID
mbr-cli query --list --limit 10             # Limit results
mbr-cli query 123                           # Execute question by ID
mbr-cli query 123 --format json             # Output as JSON
mbr-cli query 123 --format csv              # Output as CSV
mbr-cli query 123 --param date=2024-01-01   # Execute with parameters
mbr-cli query 123 --columns "name,email"    # Show specific columns
mbr-cli query 123 --offset 10              # Skip first 10 rows
mbr-cli query 123 --full                    # Show all results without pagination
mbr-cli query 123 --no-fullscreen           # Disable interactive fullscreen mode
```

### Collection

```bash
mbr-cli collection list                   # List all collections
mbr-cli collection list --format json     # Output as JSON
mbr-cli collection list --format csv      # Output as CSV
```

### Database

```bash
mbr-cli database list                     # List all databases
mbr-cli database list --format json       # Output as JSON
mbr-cli database list --format csv        # Output as CSV
```

### Global Options

- `--verbose, -v` -- Enable verbose output for debugging
- `--config-dir <DIR>` -- Override default configuration directory

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
| `PgUp` / `PgDn` | Scroll page up/down |
| `n` / `p` | Next / Previous page |
| `Home` / `g` | First page / First item |
| `End` / `G` | Last page / Last item |
| `Enter` | Execute query / Record detail |
| `/` | Search |
| `s` | Sort (result view) |
| `f` / `F` | Filter / Clear filter (result view) |
| `c` | Copy record(s) (result view) |
| `Space` | Toggle row selection |
| `Shift+Up/Down` | Range selection |
| `Shift+Home/End` | Range select to first/last |
| `Shift+PgUp/PgDn` | Range select by page |
| `Ctrl+A` | Select all rows |

## Environment Variables

| Variable | Description | Required |
|----------|-------------|----------|
| `MBR_USERNAME` | Metabase username (for non-interactive login) | No |
| `MBR_PASSWORD` | Metabase password (for non-interactive login) | No |
| `MBR_URL` | Metabase server URL (alternative to config file) | No |

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

# Format and lint
cargo fmt
cargo clippy --all-targets --all-features

# Full local check (format + lint + test)
make check
```

## License

Licensed under the MIT License.
See [LICENSE](LICENSE) file for details.
