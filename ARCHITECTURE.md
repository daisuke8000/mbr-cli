# MBR-CLI Architecture Design

## System Architecture Overview

```mermaid
graph TB
    subgraph Presentation["Presentation Layer"]
        CLI[mbr-cli]
        TUI[mbr-tui]
    end

    subgraph Core["Core Layer - mbr-core"]
        Services[Service Layer]
        API[API Client]
        Storage[Storage]
        Display[Display]
        Utils[Utils]
    end

    subgraph External["External Systems"]
        Metabase[Metabase API]
        FileSystem[Config Files]
        EnvVar[MBR_API_KEY]
    end

    CLI --> Services
    TUI --> Services

    Services --> API
    Services --> Storage
    Services --> Display

    API --> Metabase
    Storage --> FileSystem
    Storage --> EnvVar
```

## Workspace Structure

The project is organized as a Cargo Workspace with three crates:

### 1. mbr-core (Foundation Library)

The shared library containing all business logic, API communication, and data management.

**Modules:**
- `api/` - Metabase HTTP client and data models
- `core/services/` - Business logic services (ConfigService, QuestionService)
- `storage/` - Configuration (TOML) and credential management
- `utils/` - Validation, text formatting, data helpers
- `display/` - Table rendering, pagination, progress indicators
- `error.rs` - Hierarchical error system

### 2. mbr-cli (Command Line Interface)

Thin CLI wrapper using clap for argument parsing.

**Modules:**
- `cli/main_types.rs` - Command definitions with clap derive
- `cli/dispatcher.rs` - Facade delegating to services
- `cli/command_handlers.rs` - Config, Query handlers
- `cli/interactive_display.rs` - Paginated output

### 3. mbr-tui (Terminal User Interface)

Interactive TUI using ratatui framework.

**Modules:**
- `app.rs` - Application state and event loop
- `components/` - UI components (content views, modals, status bar)
- `event.rs` - Keyboard/mouse event handling
- `action.rs` - User action definitions
- `service.rs` - API service integration

## Authentication Flow

```mermaid
sequenceDiagram
    participant User
    participant App
    participant Storage
    participant Client
    participant API

    User->>App: Set MBR_API_KEY env var
    App->>Storage: get_api_key()
    Storage-->>App: API Key
    App->>Client: new(url, api_key)
    Client->>API: GET /api/user/current
    API-->>Client: User info
    Client-->>App: Authenticated
```

**Key Points:**
- Authentication is stateless via `MBR_API_KEY` environment variable
- No session management or token storage
- API key is passed with each request as `x-api-key` header

## Error Handling Hierarchy

```mermaid
graph TD
    AppError[AppError] --> CliError
    AppError --> ApiError
    AppError --> ConfigError
    AppError --> AuthError
    AppError --> StorageError
    AppError --> DisplayError
    AppError --> QuestionError
    AppError --> ServiceError
    AppError --> UtilsError

    CliError --> C1[AuthRequired]
    CliError --> C2[InvalidArguments]
    CliError --> C3[NotImplemented]

    ApiError --> A1[Timeout]
    ApiError --> A2[Http]
    ApiError --> A3[Unauthorized]

    AuthError --> AU1[MissingApiKey]
    AuthError --> AU2[ApiKeyInvalid]
    AuthError --> AU3[AuthFailed]

    ConfigError --> CF1[FileNotFound]
    ConfigError --> CF2[MissingField]
    ConfigError --> CF3[InvalidValue]
```

**Features:**
- Domain-specific error variants with context fields
- Severity levels (Critical, High, Medium, Low)
- Troubleshooting hints for common errors
- Automatic conversion via `thiserror` derive

## TUI Architecture

The TUI follows a unidirectional data flow pattern:

```mermaid
flowchart TD
    Init[App::new] --> Events

    Events[Events] --> Update[Update State]
    Update --> Render[Render Frame]
    Render --> Draw[Draw to Terminal]
    Draw --> Events

    Update --> Action[Trigger Action]
    Action --> Service[Service Call]
    Service --> Core[mbr-core API]
    Core --> Result[Result]
    Result --> Events
```

**Components:**
- `App` - Centralized state with `should_quit`, `active_tab`, `data`
- `ContentPanel` - Main content area with tabs
- `StatusBar` - Connection status and keybindings
- `HelpOverlay` - Modal help display
- `RecordDetailOverlay` - Record inspection view

## Configuration Management

```mermaid
flowchart LR
    subgraph Input["Input Sources"]
        CLI_Args[CLI Arguments]
        Env[Environment]
        TOML[config.toml]
    end

    subgraph Resolution["Resolution"]
        Parser[Arg Parser]
        Resolver[Config Resolver]
    end

    subgraph Priority["Priority Order"]
        P1[1. CLI --api-key]
        P2[2. MBR_API_KEY env]
        P3[3. config.toml]
    end

    CLI_Args --> Parser
    Env --> Resolver
    TOML --> Resolver

    Parser --> P1
    Resolver --> P2
    Resolver --> P3
```

**Configuration File:** `~/.config/mbr-cli/config.toml`

```toml
[profiles.default]
url = "https://metabase.example.com"

[profiles.production]
url = "https://metabase.prod.example.com"
```

## Query Execution Flow

```mermaid
flowchart TD
    Start([query 123]) --> CheckAuth{API Key Set?}
    CheckAuth -->|No| AuthError[Show Auth Error]
    CheckAuth -->|Yes| GetQuestion[Get Question Details]

    GetQuestion --> HasParams{Parameters?}
    HasParams -->|No| Execute[Execute Question]
    HasParams -->|Yes| ParseParams[Parse --param args]

    ParseParams --> ValidParams{Valid?}
    ValidParams -->|No| ParamError[Parameter Error]
    ValidParams -->|Yes| Execute

    Execute --> Progress[Show Progress]
    Progress --> APICall[POST /api/card/:id/query]
    APICall --> Result{Success?}

    Result -->|Yes| Format[Format Output]
    Result -->|No| APIError[Handle Error]

    Format --> Display[Display Table/JSON/CSV]
    Display --> End([Complete])

    AuthError --> End
    ParamError --> End
    APIError --> End
```

## Architecture Principles

### Layer Dependencies

Each layer only depends on layers below it:

```
CLI/TUI --> Core --> Storage --> Utils
                       |
                       v
                      API
```

### Design Patterns

- **Facade Pattern**: CLI dispatcher delegates to services
- **Service Layer**: Business logic separated from presentation
- **Component-Based UI**: TUI uses reusable components
- **Error-First Design**: Comprehensive error handling with hints

### Key Dependencies

| Crate | Purpose |
|-------|---------|
| clap 4.5 | CLI argument parsing |
| tokio 1.40 | Async runtime |
| reqwest 0.11 | HTTP client |
| ratatui 0.29 | Terminal UI framework |
| thiserror 1.0 | Error type derivation |
| serde 1.0 | Serialization |
