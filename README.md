# mbr-cli

Rust-based CLI tool for Metabase API interaction.

## Features

- **Authentication**: Login/logout with secure session management
- **Configuration**: Multi-profile support with TOML configuration
- **Secure Storage**: Keyring integration for credentials
- **Async Operations**: Built on tokio for efficient I/O

## [WIP]Quick Start

```bash
# Build
cargo build --release

# Configure
mkdir -p ~/.config/mbr-cli
# Add config.toml with metabase_url

# Login
mbr-cli auth login

# Check status
mbr-cli auth status

# Show config
mbr-cli config show
```

## [WIP]License

TBD

## Acknowledgments

- mb-cli-rs reference implementation
- Metabase API
