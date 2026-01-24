# CODEBUDDY.md

This file provides guidance to CodeBuddy Code when working with code in this repository.

## Development Commands

### Core Development
- `npm run tauri dev` - Start development mode (frontend + Tauri window with hot reload)
- `npm run dev` - Start Vite frontend development server only
- `npm run build` - Build frontend for production (includes TypeScript type checking via `vue-tsc --noEmit`)
- `npm run tauri build` - Build complete application for release

### Release Management
- `npm run version` - Increment package version without git tag
- `npm run release:patch` - Create patch release (auto increment version, commit, tag, and push)
- `npm run release:minor` - Create minor release
- `npm run release:major` - Create major release

## Architecture Overview

SSH Assistant is a Tauri-based desktop application with a Rust backend and Vue 3 + TypeScript frontend. The application uses an actor pattern for SSH session management and stores data locally in SQLite.

### Frontend Architecture (Vue 3 + TypeScript)
- **State Management**: Pinia stores handle sessions, connections, settings, notifications, SSH keys, and transfers
- **Component Structure**: Components are organized by feature (TerminalView, FileManager, AIAssistant, etc.)
- **Internationalization**: Vue i18n with English/Chinese support, configured in `src/i18n/`
- **Terminal**: xterm.js with addons for fit, search, web-links, and zmodem transfer
- **Performance**: Virtual scrolling (@tanstack/vue-virtual) for file lists

### Backend Architecture (Rust + Tauri)
- **SSH Operations**: Uses ssh2 crate for SSH connections and SFTP file operations
- **Session Management**: Actor pattern via `SshManager` that processes commands through message passing (mpsc channels)
- **Client Types**: Supports both SSH connections and WSL (Windows Subsystem for Linux)
- **Database**: SQLite with rusqlite for local persistence of connections, settings, groups, and SSH keys
- **Async Runtime**: tokio for asynchronous operations
- **Command Pattern**: Tauri commands (`#[tauri::command]`) expose Rust functionality to the frontend

### Key Architectural Patterns

**Session Management Flow**:
1. Frontend calls `connect` command with connection config
2. Backend establishes SSH connection (with jump host support if configured)
3. `SshManager` actor is spawned to handle session operations
4. Frontend receives session ID and stores in session store
5. All subsequent operations (shell, SFTP, commands) are sent as `SshCommand` messages to the manager

**Communication Pattern**:
- Frontend → Backend: Tauri commands (synchronous-looking but can be async)
- Backend Internal: Actor pattern with message passing via `mpsc` channels
- Progress Events: Tauri events (`emit`) for real-time file transfer progress

**Connection Types**:
- SSH: Uses `ssh2` crate with password or SSH key authentication
- WSL: Direct shell access to Windows Subsystem for Linux distros

## Project Structure

### Frontend (`/src/`)
- `components/` - Vue components organized by feature
  - `TerminalView.vue` - xterm.js terminal interface
  - `FileManager.vue` - SFTP file browser with drag-and-drop
  - `AIAssistant.vue` - AI chat interface with command execution
  - `SessionTabs.vue` - Multi-tab session management
  - `ConnectionList.vue` - Connection tree with group support
- `stores/` - Pinia state management
  - `sessions.ts` - Active SSH sessions and workspace indexing
  - `connections.ts` - Connection configurations and groups
  - `settings.ts` - App settings (theme, AI, terminal appearance)
  - `transfers.ts` - File transfer tracking
  - `sshKeys.ts` - SSH key management
  - `notifications.ts` - Toast notifications
- `composables/` - Reusable Vue composition functions (i18n, path handling, file icons)
- `types.ts` - TypeScript interfaces for shared types between frontend/backend
- `i18n/` - Internationalization resources (en.json, zh.json)

### Backend (`/src-tauri/src/`)
- `lib.rs` - Main entry point, Tauri app setup, command registration
- `models.rs` - Data structures serialized between frontend/backend
- `db.rs` - SQLite database operations and migrations
- `ssh/mod.rs` - SSH module with re-exports and constants
- `ssh/manager.rs` - `SshManager` actor that processes session commands
- `ssh/client.rs` - High-level SSH client wrapper and session management
- `ssh/connection.rs` - Low-level SSH connection establishment and session pool
- `ssh/terminal.rs` - PTY shell management
- `ssh/command.rs` - Single command execution
- `ssh/file_ops.rs` - SFTP file operations with progress tracking
- `ssh/keys.rs` - SSH key operations
- `ssh/utils.rs` - Retry logic and error handling helpers
- `ssh/wsl.rs` - WSL-specific connection handling
- `system.rs` - File icon resolution utilities

## Important Implementation Details

### TypeScript Configuration
- Strict mode enabled with `noUnusedLocals` and `noUnusedParameters` enforced
- Build process includes type checking via `vue-tsc --noEmit`
- Shared types defined in `src/types.ts` correspond to Rust structs in `src-tauri/src/models.rs`

### SSH Connection Flow
- Connections can use password or SSH key authentication
- Jump host (bastion) connections are supported through SSH tunneling
- Session pooling allows background operations without disrupting active shell
- Connection timeouts are configurable (DEFAULT_CONNECTION_TIMEOUT: 15s, JUMP_HOST_TIMEOUT: 30s)

### File Transfer System
- File transfers are tracked with unique IDs and can be cancelled
- Progress is emitted via Tauri events with throttled updates for performance
- Configurable SFTP buffer size (default: 512KB) for optimal performance
- Resume capability with file integrity verification

### Workspace Indexing
- AI assistant can index remote workspaces by:
  - Generating file tree (using `tree -L 2` or `find` command)
  - Reading key config files (package.json, Cargo.toml, etc.)
  - Checking git status
- Context is provided to AI for intelligent command suggestions

### Session Lifecycle
- Session IDs are UUIDs generated by backend
- Sessions have three states: `connected`, `disconnected`, `connecting`
- Optimistic UI updates for better UX (disconnect removes from UI immediately)
- Reconnect reuses existing session ID

## Build Requirements

- Node.js (v16+)
- Rust toolchain (for Tauri backend compilation)
- On Windows, may require additional build tools for Tauri

## Key Dependencies

**Frontend**: Vue 3, TypeScript, Pinia, TailwindCSS, xterm.js, Monaco Editor, Lucide Icons, @tanstack/vue-virtual

**Backend**: Tauri v2, ssh2, rusqlite, tokio

**Tauri Plugins**: dialog, filesystem, opener
