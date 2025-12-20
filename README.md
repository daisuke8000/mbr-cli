# mbr-cli

Rust-based CLI/TUI tool for Metabase API interaction.

## Features

- **API Key Authentication**: Simple authentication via `MBR_API_KEY` environment variable
- **Multi-Profile Configuration**: TOML-based configuration with profile support
- **CLI Interface**: Command-line tool for scripting and quick operations
- **Rich TUI Experience**: Interactive terminal UI with keyboard navigation (`mbr-tui`)
- **Hierarchical Error Handling**: Comprehensive error system with troubleshooting hints
- **Async Operations**: Built on tokio for efficient I/O operations

## Architecture

This project uses a **Cargo Workspace** with 3 crates:

```
crates/
├── mbr-core/     # Shared library - API client, storage, business logic
├── mbr-cli/      # CLI binary - clap-based command interface
└── mbr-tui/      # TUI binary - ratatui-based interactive interface
```

**Dependency Flow:**
```
mbr-cli  ──┐
           ├──► mbr-core
mbr-tui  ──┘
```

See `ARCHITECTURE.md` for detailed system diagrams and implementation details.

## Quick Start

### 1. Set up authentication

```bash
# Set your Metabase API key
export MBR_API_KEY="your_api_key_here"
```

Generate an API key in Metabase: Settings → Admin settings → API Keys

### 2. Configure your Metabase URL

```bash
# Set via command line
cargo run -p mbr-cli -- config set --url "https://your-metabase.com"
```

### 3. Validate connection

```bash
# Verify API key and connection
cargo run -p mbr-cli -- config validate
```

### 4. Start using

```bash
# CLI: List available questions
cargo run -p mbr-cli -- query --list

# CLI: Execute a question
cargo run -p mbr-cli -- query 123

# TUI: Launch interactive terminal UI
cargo run -p mbr-tui
```

## CLI Commands

### Configuration
```bash
mbr-cli config show                              # Display current configuration
mbr-cli config set --url "https://metabase.com"  # Set Metabase URL
mbr-cli config validate                          # Validate API key and connection
```

### Query
```bash
mbr-cli query --list                        # List all questions
mbr-cli query --list --search "sales"       # Search questions
mbr-cli query 123                           # Execute question by ID
mbr-cli query 123 --format json             # Output as JSON
mbr-cli query 123 --param date=2024-01-01   # Execute with parameters
```

### Global Options
- `--verbose, -v` - Enable verbose output
- `--profile, -p <PROFILE>` - Use specific profile (defaults to 'default')
- `--config-dir <DIR>` - Override default config directory
- `--api-key <KEY>` - Set API key (also via `MBR_API_KEY` environment variable)

## TUI Controls

| Key | Action |
|-----|--------|
| `1`, `2`, `3` | Switch tabs (Questions, Collections, Databases) |
| `↑/↓` or `j/k` | Navigate items |
| `Enter` | Execute/Select |
| `?` | Show help |
| `q` | Quit |

## Environment Variables

| Variable | Description | Required |
|----------|-------------|----------|
| `MBR_API_KEY` | Metabase API key for authentication | Yes |

## Development

```bash
# Build all crates
cargo build

# Run tests
cargo test

# Run specific crate
cargo run -p mbr-cli -- --help
cargo run -p mbr-tui
```

## License

Licensed under the MIT License.
See [LICENSE](LICENSE) file for details.

## Resources

- [Metabase API Documentation](https://www.metabase.com/docs/latest/api-documentation.html)
- [Metabase API Keys Guide](https://www.metabase.com/docs/latest/people-and-groups/api-keys.html)
