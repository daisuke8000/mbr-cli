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
        FileSystem[Config Files / Session]
        EnvVar[MBR_USERNAME / MBR_PASSWORD / MBR_URL]
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
- `storage/` - Configuration (TOML) and session credential management (session.json)
- `utils/` - Validation, text formatting, data helpers
- `display/` - Table rendering, pagination, progress indicators
- `error.rs` - Hierarchical error system

### 2. mbr-cli (Command Line Interface)

Thin CLI wrapper using clap for argument parsing.

**Modules:**
- `cli/main_types.rs` - Command definitions with clap derive
- `cli/dispatcher.rs` - Facade delegating to services
- `cli/command_handlers.rs` - Config, Query, Collection, Database handlers
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

### Login Flow

```mermaid
sequenceDiagram
    participant User
    participant App
    participant Storage
    participant Client
    participant API

    User->>App: mbr-cli login
    App->>User: Prompt username/password
    Note over App: Or read MBR_USERNAME/MBR_PASSWORD env vars
    App->>Client: login(url, username, password)
    Client->>API: POST /api/session
    API-->>Client: { id: "session-token" }
    Client-->>App: Session token
    App->>Storage: save_session(session.json)
    Storage-->>App: Saved
    App-->>User: Login successful
```

### Authenticated Request Flow

```mermaid
sequenceDiagram
    participant App
    participant Storage
    participant Client
    participant API

    App->>Storage: load_session()
    Storage-->>App: Session { token, url, username }
    App->>Client: with_session_token(url, token)
    Client->>API: Request with X-Metabase-Session header
    API-->>Client: Response
    Note over App,API: On 401 Unauthorized, auto re-login if MBR_USERNAME/MBR_PASSWORD set
```

### Logout Flow

```mermaid
sequenceDiagram
    participant User
    participant App
    participant Storage
    participant Client
    participant API

    User->>App: mbr-cli logout
    App->>Storage: load_session()
    App->>Client: DELETE /api/session
    App->>Storage: delete_session()
    App-->>User: Logged out
```

**Key Points:**
- Session-based authentication via `mbr-cli login` command
- Session token stored at `~/.config/mbr-cli/session.json`
- Each API request includes `X-Metabase-Session` header with the session token
- Auto re-login on 401 if `MBR_USERNAME` and `MBR_PASSWORD` environment variables are set
- Credentials can be provided interactively (prompt) or via environment variables

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

    AuthError --> AU1[NotLoggedIn]
    AuthError --> AU2[SessionExpired]
    AuthError --> AU3[LoginFailed]

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

    subgraph Priority["Priority Order (URL)"]
        P1[1. MBR_URL env]
        P2[2. config.toml]
    end

    subgraph Auth["Authentication"]
        S1[Session token from session.json]
        S2[Auto re-login via MBR_USERNAME/MBR_PASSWORD]
    end

    CLI_Args --> Parser
    Env --> Resolver
    TOML --> Resolver

    Resolver --> P1
    Resolver --> P2
```

**Configuration File:** `~/.config/mbr-cli/config.toml`

```toml
url = "https://metabase.example.com"
```

**Session File:** `~/.config/mbr-cli/session.json`

```json
{
  "session_token": "...",
  "url": "https://metabase.example.com",
  "username": "user@example.com",
  "created_at": "2026-03-30T12:00:00Z"
}
```

**URL Priority Order:**
1. `MBR_URL` environment variable
2. `config.toml` file

**Authentication:**
- Session token loaded from `~/.config/mbr-cli/session.json`
- Created by `mbr-cli login` command
- Auto re-login on 401 if `MBR_USERNAME` and `MBR_PASSWORD` are set

## Query Execution Flow

```mermaid
flowchart TD
    Start([query 123]) --> CheckAuth{Session Valid?}
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
