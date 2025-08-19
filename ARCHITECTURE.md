# MBR-CLI Architecture Design

## System Architecture Overview

```mermaid
graph TB
    subgraph "CLI Layer"
        CLI[main.rs]
        Auth[auth commands]
        Config[config commands]
        Question[question commands]
    end
    
    subgraph "Core Layer"
        AuthCore[Authentication Service]
        ProfileCore[Profile Management]
        QuestionCore[Question Service]
    end
    
    subgraph "Storage Layer"
        ConfigStore[Config Files]
        SecureStore[Keyring Storage]
        SessionStore[Session Cache]
    end
    
    subgraph "Utils Layer"
        Interactive[Interactive Input]
        Display[Table Display]
        Progress[Progress Indicators]
    end
    
    subgraph "External"
        Metabase[Metabase API]
        FileSystem[~/.config/mbr-cli/]
        Keychain[System Keychain]
    end
    
    CLI --> AuthCore
    CLI --> ProfileCore
    CLI --> QuestionCore
    
    AuthCore --> SecureStore
    ProfileCore --> ConfigStore
    QuestionCore --> Metabase
    
    ConfigStore --> FileSystem
    SecureStore --> Keychain
    
    AuthCore --> Interactive
    QuestionCore --> Display
    QuestionCore --> Progress
    
    %% Error Flow
    AuthCore -.-> ErrorHandler[Error Handler]
    ProfileCore -.-> ErrorHandler
    QuestionCore -.-> ErrorHandler
    ErrorHandler --> CLI
```

## Authentication Flow

```mermaid
sequenceDiagram
    participant User
    participant CLI
    participant AuthCore
    participant SecureStore
    participant Metabase
    participant Keychain
    
    Note over User,Keychain: Initial Login
    User->>CLI: mbr-cli auth login
    CLI->>AuthCore: authenticate()
    AuthCore->>User: Prompt: Enter API Key
    User->>AuthCore: API Key
    AuthCore->>Metabase: POST /api/session
    Metabase-->>AuthCore: session token
    AuthCore->>SecureStore: save_session(token)
    SecureStore->>Keychain: secure_store(token)
    AuthCore-->>CLI: Authentication Success
    CLI-->>User: "Login successful"
    
    Note over User,Keychain: Authenticated Operations
    User->>CLI: mbr-cli question list
    CLI->>AuthCore: check_authentication()
    AuthCore->>SecureStore: get_session()
    SecureStore->>Keychain: retrieve_token()
    Keychain-->>SecureStore: session_token
    SecureStore-->>AuthCore: valid_session
    AuthCore->>Metabase: GET /api/card (with token)
    Metabase-->>AuthCore: questions_data
    AuthCore-->>CLI: questions
    CLI-->>User: Display table format
    
    Note over User,Keychain: Session Expired
    User->>CLI: mbr-cli question execute 123
    CLI->>AuthCore: check_authentication()
    AuthCore->>Metabase: validate session
    Metabase-->>AuthCore: 401 Unauthorized
    AuthCore->>SecureStore: clear_session()
    AuthCore-->>CLI: AuthRequired Error
    CLI-->>User: "Please login again: mbr-cli auth login"
```

## Question Execution Flow

```mermaid
flowchart TD
    Start([mbr-cli question execute 123]) --> Auth{Check Auth}
    Auth -->|Not Authenticated| AuthError[Show Auth Error]
    Auth -->|Authenticated| GetQ[Get Question Details]
    
    GetQ --> CheckParams{Parameters Required?}
    CheckParams -->|No| ExecQ[Execute Question]
    CheckParams -->|Yes| ParseParams[Parse Parameters]
    
    ParseParams --> ValidParams{Valid Parameters?}
    ValidParams -->|Invalid| ParamError[Parameter Error]
    ValidParams -->|Valid| ExecQ
    
    ExecQ --> Progress[Start Progress Display]
    Progress --> APICall[Call Metabase API]
    APICall --> APIResult{API Result}
    
    APIResult -->|Success| FormatResult[Format Results]
    APIResult -->|Error| APIError[Handle API Error]
    
    FormatResult --> DisplayTable[Display Table]
    DisplayTable --> End([Complete])
    
    AuthError --> End
    ParamError --> End
    APIError --> End
    
    style Start fill:#e1f5fe
    style End fill:#f3e5f5
    style Auth fill:#fff3e0
    style ExecQ fill:#e8f5e8
```

## Configuration Management Flow

```mermaid
stateDiagram-v2
    [*] --> CheckConfig: App Start
    
    CheckConfig --> ConfigExists: Check Config File
    ConfigExists --> LoadConfig: ~/.config/mbr-cli/config.toml exists
    ConfigExists --> CreateDefault: Config file missing
    
    CreateDefault --> DefaultCreated: Create default config
    DefaultCreated --> LoadConfig
    
    LoadConfig --> ValidateConfig: Validate config
    ValidateConfig --> ConfigReady: Validation success
    ValidateConfig --> ConfigError: Validation failed
    
    ConfigReady --> ResolveProfile: Resolve profile
    ResolveProfile --> EnvOverride: Environment override
    EnvOverride --> FinalConfig: Final config
    
    ConfigError --> ShowError: Show error message
    ShowError --> [*]
    
    FinalConfig --> [*]: Config complete
    
    note right of CreateDefault
        Default config:
        - development profile
        - localhost:3000
        - table output format
    end note
    
    note right of EnvOverride
        Environment priority:
        1. MBR_API_KEY
        2. MBR_PROFILE
        3. MBR_CONFIG_DIR
    end note
```

## Error Handling Hierarchy

```mermaid
graph TD
    AppError[AppError<br/>Top Level Error] --> CliError[CliError<br/>CLI Operation Error]
    AppError --> ApiError[ApiError<br/>API Communication Error]
    AppError --> ConfigError[ConfigError<br/>Configuration Error]
    AppError --> AuthError[AuthError<br/>Authentication Error]
    AppError --> StorageError[StorageError<br/>Storage Error]
    AppError --> QuestionError[QuestionError<br/>Question Operation Error]
    
    CliError --> AuthRequired[Authentication Required]
    CliError --> InvalidArgs[Invalid Arguments]
    
    ApiError --> Timeout[Timeout]
    ApiError --> Http[HTTP Error]
    ApiError --> Unauthorized[Authentication Failed]
    ApiError --> RateLimit[Rate Limited]
    
    ConfigError --> FileNotFound[Config File Not Found]
    ConfigError --> InvalidFormat[Invalid Format]
    ConfigError --> MissingField[Missing Required Field]
    
    AuthError --> InvalidCredentials[Invalid Credentials]
    AuthError --> SessionExpired[Session Expired]
    
    StorageError --> KeyringError[Keychain Error]
    StorageError --> FilePermission[File Permission Error]
    
    style AppError fill:#ffcdd2
    style CliError fill:#fff3e0
    style ApiError fill:#e8f5e8
    style ConfigError fill:#e1f5fe
    style AuthError fill:#fce4ec
    style StorageError fill:#f3e5f5
```

## Data Flow Overview

```mermaid
flowchart LR
    subgraph Input
        CLI_Args[CLI Arguments]
        Env_Vars[Environment Variables]
        Config_File[Config File]
    end
    
    subgraph Processing
        Parser[Argument Parser]
        Resolver[Config Resolver]
        Validator[Validation]
        Executor[Command Executor]
    end
    
    subgraph Output
        Table[Table Display]
        JSON[JSON Output]
        Error_Msg[Error Messages]
    end
    
    subgraph External
        Metabase_API[Metabase API]
        System_Keychain[System Keychain]
    end
    
    CLI_Args --> Parser
    Env_Vars --> Resolver
    Config_File --> Resolver
    
    Parser --> Validator
    Resolver --> Validator
    Validator --> Executor
    
    Executor <--> Metabase_API
    Executor <--> System_Keychain
    
    Executor --> Table
    Executor --> JSON
    Executor --> Error_Msg
    
    style Processing fill:#e8f5e8
    style External fill:#fff3e0
```

## Architecture Principles

### Layer Dependencies
- **CLI Layer**: User interface, argument parsing
- **Core Layer**: Business logic, domain services
- **Storage Layer**: Data persistence, secure storage
- **Utils Layer**: Common utilities, display functions

### Error-First Design
1. **Failure Pattern First**: Design all possible failure cases upfront
2. **Hierarchical Error Handling**: Domain-specific errors → unified errors
3. **Usability Focus**: Practical error messages and recovery procedures

### Async-First Architecture
- **tokio runtime**: All I/O operations handled asynchronously
- **reqwest**: Async HTTP communication execution
- **async/await**: Explicit async boundary management

## Current Implementation Status

### Implemented Components ✅
- **Error System** (src/error.rs): Full hierarchy with AppError, CliError, ApiError, ConfigError, AuthError, StorageError
- **CLI Layer**:
  - main_types.rs: Command structure with clap derive macros
  - dispatcher.rs: Command routing with auth login/logout, config show
- **Storage Layer**:
  - config.rs: TOML configuration with profile management
  - credentials.rs: Keyring integration with session persistence
- **API Layer**:
  - client.rs: MetabaseClient with login/logout/session management
  - models.rs: API data models with custom deserializers

### In Progress 🔄
- **Session Management**: Auto-restoration on startup
- **Question Commands**: List and execute operations

### Not Implemented ⏳
- **Core Layer**: Business logic services
- **Utils Layer**: Display utilities, progress indicators
- **Config Commands**: Set operations
- **Cache System**: Response caching mechanism