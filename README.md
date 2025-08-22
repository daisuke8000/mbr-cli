# mbr-cli

Rust-based CLI tool for Metabase API interaction.

## Features

- **Authentication**: Login/logout with secure session management via service layer
- **Configuration**: Multi-profile support with TOML configuration and validation  
- **Secure Storage**: Keyring integration for credentials with dual-mode authentication
- **Interactive Display**: Full-screen pagination and interactive table navigation
- **Error Handling**: Hierarchical error system with comprehensive user feedback
- **Async Operations**: Built on tokio for efficient I/O operations

## Architecture

**4-Layer Clean Architecture** with clear dependency flow:

```
CLI Layer (User Interface)
├── Core Layer (Business Logic) 
├── Storage Layer (Data Persistence)
└── Utils Layer (Shared Utilities)
```

**Key Architectural Features**:
- **Service Layer Pattern**: Business logic separated from CLI using facade pattern
- **Single Responsibility**: Each module has focused, well-defined responsibilities
- **Dependency Inversion**: Each layer only depends on layers below it
- **Error-First Design**: Comprehensive failure handling with hierarchical error types
- **Zero Circular Dependencies**: Clean module boundaries validated by architecture tests

See `ARCHITECTURE.md` for detailed system diagrams and implementation details.

## Quick Start

### Development (cargo run)
```bash
# View help and available commands
cargo run -- --help

# Initialize configuration (creates default config automatically)
cargo run -- config show

# Authenticate with your Metabase instance
cargo run -- auth login

# Verify authentication status
cargo run -- auth status
```

### Production (release build)
```bash
# Build the project
cargo build --release

# View current configuration (creates default profile if none exists)
./target/release/mbr-cli config show

# Authenticate with Metabase
./target/release/mbr-cli auth login

# Verify authentication status
./target/release/mbr-cli auth status
```

### Custom Configuration
The application automatically creates a default profile (`default`) pointing to `http://localhost:3000`. To customize your Metabase URL:

```bash
# Edit the configuration file manually
mkdir -p ~/.config/mbr-cli
cat > ~/.config/mbr-cli/config.toml << EOF
[default]
url = "https://your-metabase.com"
email = "your-email@example.com"
EOF
```

## Available Commands

### Authentication (Implemented)
- `auth login` - Login with username/password (supports CLI args, environment variables, and interactive input)  
- `auth logout` - Clear stored session and credentials
- `auth status` - Display current authentication status and profile information

**Environment Variables for Authentication:**
- `MBR_USERNAME` - Username for login (fallback when --username not provided)
- `MBR_PASSWORD` - Password for login (fallback when --password not provided)

### Configuration (Implemented)  
- `config show` - Display current configuration and all profiles
- `config set` - Set configuration values (supports multiple input modes)

**Config Set Usage:**
```bash
# Using environment variables (MBR_URL and MBR_USERNAME)
export MBR_URL="https://your-metabase.com"
export MBR_USERNAME="your-email@example.com"
mbr-cli config set

# Using CLI arguments (takes priority over env vars)
mbr-cli config set --url "https://metabase.com" --email "user@example.com"

# Legacy field/value mode (backward compatible)
mbr-cli config set --field "url" --value "https://metabase.com"
```

### Questions (Implemented)
- `question list` - List available questions with search, limit, and collection filtering
- `question execute <id>` - Execute question with parameters and display results with pagination

### Global Options
Available for all commands:
- `--verbose, -v` - Enable verbose output
- `--profile, -p <PROFILE>` - Use specific profile (defaults to 'default')
- `--config-dir <DIR>` - Override default config directory
- `--api-key <KEY>` - Set API key (also via `MBR_API_KEY` environment variable)

### Environment Variables Summary
- `MBR_API_KEY` - API key for authentication (alternative to username/password)
- `MBR_USERNAME` - Username for login and profile email configuration
- `MBR_PASSWORD` - Password for login
- `MBR_URL` - Metabase server URL for profile configuration

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.  
See [LICENSE](LICENSE) file for details.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this work by you shall be dual licensed as above, without any additional terms or conditions.

## Resources

- [Metabase API Documentation](https://www.metabase.com/docs/latest/api-documentation.html)
- [Metabase Authentication Guide](https://www.metabase.com/docs/latest/people-and-groups/start.html#authentication)
