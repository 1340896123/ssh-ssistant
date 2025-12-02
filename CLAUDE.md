# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

SSH Assistant is a modern desktop SSH client built with Tauri (Rust backend) + Vue 3 (TypeScript frontend). It provides integrated terminal functionality, AI assistant capabilities, and a full-featured file manager for remote server management.

## Development Commands

### Core Development
- `npm run tauri dev` - Start development mode (frontend + Tauri window with hot reload)
- `npm run dev` - Start Vite frontend development server only
- `npm run build` - Build frontend for production (includes TypeScript type checking)
- `npm run tauri build` - Build complete application for release

### Release Management
- `npm run release:patch` - Create patch release (auto increment version and tag)
- `npm run release:minor` - Create minor release
- `npm run release:major` - Create major release
- `npm run version` - Increment package version without git tag

## Architecture

### Frontend (Vue 3 + TypeScript)
- **Framework**: Vue 3 with Composition API
- **State Management**: Pinia stores for sessions, connections, settings, notifications
- **UI**: TailwindCSS with custom components
- **Terminal**: xterm.js with addons (fit, search, web-links, zmodem)
- **Internationalization**: Vue i18n with English/Chinese support
- **Virtual Scrolling**: @tanstack/vue-virtual for file lists

### Backend (Rust + Tauri)
- **SSH Operations**: ssh2 crate for connections and SFTP file operations
- **Database**: SQLite with rusqlite for local persistence
- **File System**: Tauri filesystem plugin for local file operations
- **Async Runtime**: tokio for asynchronous operations
- **Command Pattern**: Tauri commands for frontend-backend communication

## Key Directories

### Frontend Structure (`/src/`)
- `components/` - Vue components (TerminalView, FileManager, AIAssistant, etc.)
- `stores/` - Pinia state management (sessions, connections, settings, notifications)
- `i18n/` - Internationalization resources and locale files
- `composables/` - Reusable Vue composition functions
- `types/` - TypeScript type definitions

### Backend Structure (`/src-tauri/src/`)
- `lib.rs` - Main Tauri application setup and command handlers
- `ssh.rs` - SSH connection and session management
- `db.rs` - Database operations and models
- `models.rs` - Data structures for serialization

## Key Technologies

### Core Dependencies
- **Tauri v2** - Desktop application framework
- **Vue 3** + **TypeScript** - Frontend framework
- **Vite** - Build tool and development server
- **Pinia** - State management
- **xterm.js** - Terminal emulation
- **ssh2** - SSH client implementation (Rust)
- **SQLite** - Local data persistence

### UI/UX Libraries
- **TailwindCSS** - Styling framework
- **Lucide Vue** - Icon library
- **@tanstack/vue-virtual** - Virtual scrolling performance

## Development Guidelines

### TypeScript Configuration
- Strict TypeScript mode enabled
- `noUnusedLocals` and `noUnusedParameters` enforced
- Build process includes type checking via `vue-tsc --noEmit`

### Component Patterns
- Vue 3 Composition API preferred
- TypeScript interfaces required for props and data structures
- Components follow single responsibility principle
- State management via Pinia stores for cross-component data

### Rust Backend
- Async/await patterns with tokio runtime
- Error handling using Rust's Result types
- Tauri commands for frontend communication
- SQLite models with serde serialization

## Key Features Implementation

### SSH Session Management
- Multi-tab interface with isolated sessions
- Password authentication with jump host support
- Session persistence via SQLite database
- Real-time connection status monitoring

### File Operations
- SFTP-based file management with drag-and-drop
- Configurable buffer sizes for optimal transfer performance
- File integrity verification with resume capability
- Local editing with auto-upload on file changes

### AI Assistant Integration
- Context-aware command suggestions
- Configurable API endpoints and parameters
- Direct command execution from chat interface

### Performance Optimizations
- Throttled progress updates during file transfers
- Virtual scrolling for large file lists
- Optimized SFTP buffer settings
- OS-specific terminal handling

## Build Requirements

### Prerequisites
- Node.js (v16+)
- Rust toolchain (for Tauri backend compilation)

### Development Workflow
1. Run `npm install` to install dependencies
2. Use `npm run tauri dev` for development with hot reload
3. Build with `npm run build` for frontend type checking and compilation
4. Create releases with automated `npm run release:*` commands

## Testing and Quality

- TypeScript strict mode ensures type safety
- Frontend build includes comprehensive type checking
- Rust backend leverages compiler safety guarantees
- Component-based architecture enables modular testing