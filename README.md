# mbr-cli

Rust-based CLI tool for Metabase API interaction.

## Features

- **API Key Authentication**: Simple authentication via `MBR_API_KEY` environment variable
- **Configuration**: Multi-profile support with TOML configuration and validation
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

### 1. Set up authentication

```bash
# Set your Metabase API key
export MBR_API_KEY="your_api_key_here"
```

Generate an API key in Metabase: Settings → Admin settings → API Keys

### 2. Configure your Metabase URL

```bash
# Option 1: Set via environment variable
export MBR_URL="https://your-metabase.com"
cargo run -- config set

# Option 2: Set via command line argument
cargo run -- config set --url "https://your-metabase.com"
```

### 3. Validate connection

```bash
# Verify your API key and connection
cargo run -- config validate
```

### 4. Start querying

```bash
# List available questions
cargo run -- query --list

# Execute a question
cargo run -- query 123
```

## Available Commands

### Configuration
- `config show` - Display current configuration and all profiles
- `config set` - Set configuration values (URL, email)
- `config validate` - Validate API key and test connection to Metabase

**Config Usage:**
```bash
# View current configuration
mbr-cli config show

# Set Metabase URL
mbr-cli config set --url "https://metabase.com"

# Set both URL and email
mbr-cli config set --url "https://metabase.com" --email "user@example.com"

# Validate API key and connection
mbr-cli config validate
```

### Query
- `query --list` - List available questions with search, limit, and collection filtering
- `query <id>` - Execute question with parameters and display results with pagination

**Query Usage:**
```bash
# List all questions
mbr-cli query --list

# List with search and limit
mbr-cli query --list --search "sales" --limit 10

# Execute a question by ID
mbr-cli query 123

# Execute with output format
mbr-cli query 123 --format json
mbr-cli query 123 --format csv

# Execute with parameters
mbr-cli query 123 --param date=2024-01-01 --param region=US
```

### Global Options
Available for all commands:
- `--verbose, -v` - Enable verbose output
- `--profile, -p <PROFILE>` - Use specific profile (defaults to 'default')
- `--config-dir <DIR>` - Override default config directory
- `--api-key <KEY>` - Set API key (also via `MBR_API_KEY` environment variable)

## Environment Variables

| Variable | Description | Required |
|----------|-------------|----------|
| `MBR_API_KEY` | Metabase API key for authentication | Yes |
| `MBR_URL` | Metabase server URL | No (can use config) |

## License

Licensed under the MIT License.
See [LICENSE](LICENSE) file for details.

## Resources

- [Metabase API Documentation](https://www.metabase.com/docs/latest/api-documentation.html)
- [Metabase API Keys Guide](https://www.metabase.com/docs/latest/people-and-groups/api-keys.html)
