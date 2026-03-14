# SSH Star

<div align="center">

**A Modern SSH Client for the Modern Era**

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Tauri](https://img.shields.io/badge/Tauri-2.0-FFC131?logo=tauri)](https://tauri.app/)
[![Vue](https://img.shields.io/badge/Vue-3.5-4FC08D?logo=vue.js&logoColor=white)](https://vuejs.org/)
[![Rust](https://img.shields.io/badge/Rust-2021-000000?logo=rust)](https://www.rust-lang.org/)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.6-3178C6?logo=typescript&logoColor=white)](https://www.typescriptlang.org/)

[English](README.md) | [简体中文](README.zh-CN.md)

</div>

---

## ✨ Introduction

**SSH Star** is a cutting-edge SSH client built with [Tauri 2](https://tauri.app/), [Vue 3](https://vuejs.org/), and [TypeScript](https://www.typescriptlang.org/). It combines the power of native applications with the flexibility of web technologies to deliver an unparalleled server management experience.

Designed for developers, system administrators, and DevOps engineers, SSH Star provides a unified platform for SSH connections, terminal operations, file management, and AI-assisted workflows.

### 🌟 Key Highlights

- **🚀 Lightning Fast** - Built with Rust for backend and optimized Vue 3 for frontend
- **💪 Feature-Rich** - Everything you need in one application: terminal, file manager, AI assistant, tunneling
- **🎨 Modern UI** - Clean, intuitive interface with dark/light themes
- **🔒 Secure** - Local data storage with encrypted SSH keys support
- **🌐 Cross-Platform** - Works on Windows, macOS, and Linux
- **🤖 AI-Powered** - Integrated AI assistant for smart command suggestions and automation

![SSH Star Screenshot](exampleImg/image.png)

---

## Key Features

### Connection Management

**Comprehensive SSH Connection Support**

- Create, edit, and delete SSH connections with ease
- Password authentication with secure credential storage
- Jump Host/Bastion server support for complex network topologies
- Connection validation and status monitoring
- Local SQLite database for persistent connection storage

### Session Management

**Multi-Tab Interface**

- Manage multiple server sessions simultaneously
- Complete session isolation for security and stability
- Each session integrates terminal, file management, and AI assistant
- Tab reordering and quick switching

### Smart Terminal

**Advanced Terminal Experience**

- Full-featured terminal based on xterm.js
- Traditional Tab auto-completion for commands
- **AI Smart Completion**: Context-aware intelligent command suggestions
- Terminal search and web link detection
- ZModem protocol support for file transfers

### File Management

**Powerful SFTP Operations**

- Browse, upload, and download remote files
- Drag-and-drop support for seamless file transfers
- Create, delete, and rename files and directories
- **Local Editing**: Edit remote files in your local editor with auto-upload on save
- Resume interrupted transfers with file integrity verification
- Configurable buffer sizes for optimal transfer performance

### AI Assistant

**Intelligent Context-Aware Assistance**

- Natural language conversations with context awareness
- Execute terminal commands directly from the chat interface
- Configurable AI model parameters (API endpoint, key, model name)
- Smart command suggestions based on your workflow

### SSH Tunnel Management

**Comprehensive Tunneling Support**

- Local port forwarding
- Remote port forwarding
- Dynamic port forwarding (SOCKS proxy)
- Tunnel status monitoring and management

### Transfer Management

**Advanced File Transfer Control**

- Pause and resume file transfers
- Queue management for multiple transfers
- Transfer progress tracking with detailed statistics
- Automatic retry on transfer failures

### System Monitoring

**Real-Time Server Insights**

- Server status monitoring
- Disk usage analysis
- Resource utilization tracking

### Personalization

**Customizable User Experience**

- Multi-language support (English/Chinese)
- UI theme switching (Light/Dark mode)
- Custom AI completion and chat configurations
- Flexible terminal settings

### SSH Key Management

**Comprehensive Key Operations**

- Generate new SSH key pairs
- Import existing keys
- Manage multiple SSH keys
- Key-based authentication support

---

## Technology Stack

### Frontend

- **Framework**: [Vue 3](https://vuejs.org/) with Composition API
- **Language**: [TypeScript 5.6](https://www.typescriptlang.org/)
- **Build Tool**: [Vite 6](https://vitejs.dev/)
- **Styling**: [TailwindCSS](https://tailwindcss.com/)
- **State Management**: [Pinia 3](https://pinia.vuejs.org/)
- **Internationalization**: [Vue i18n](https://vue-i18n.intlify.dev/)
- **Terminal**: [xterm.js 5](https://xtermjs.org/) with addons
- **Icons**: [Lucide Vue](https://lucide.dev/)
- **Virtual Scrolling**: [@tanstack/vue-virtual](https://tanstack.com/virtual/latest)
- **Code Editor**: [Monaco Editor](https://microsoft.github.io/monaco-editor/)

### Backend

- **Framework**: [Tauri 2](https://tauri.app/) (Rust)
- **SSH Operations**: [ssh2](https://github.com/alexcrichton/ssh2-rs)
- **Database**: [SQLite](https://www.sqlite.org/) with [rusqlite](https://github.com/rusqlite/rusqlite)
- **Async Runtime**: [tokio](https://tokio.rs/)
- **File System**: Tauri filesystem plugin

---

## Architecture

```text
ssh-ssistant-tauri/
├── src/                          # Frontend Vue 3 + TypeScript
│   ├── components/               # Vue components
│   │   ├── TerminalView.vue      # Terminal interface with xterm.js
│   │   ├── FileManager.vue       # SFTP file manager
│   │   ├── AIAssistant.vue       # AI chat interface
│   │   ├── ConnectionManager.vue # Connection CRUD
│   │   └── ...                   # Other UI components
│   ├── stores/                   # Pinia state management
│   │   ├── sessions.ts           # Session state
│   │   ├── connections.ts        # Connection data
│   │   ├── settings.ts           # User preferences
│   │   └── notifications.ts      # Notification system
│   ├── i18n/                     # Internationalization
│   │   ├── locales/
│   │   │   ├── en.json           # English translations
│   │   │   └── zh-CN.json        # Chinese translations
│   ├── composables/              # Vue composition functions
│   ├── types/                    # TypeScript type definitions
│   └── App.vue                   # Root component
├── src-tauri/                    # Backend Rust + Tauri
│   ├── src/
│   │   ├── lib.rs                # Main Tauri setup
│   │   ├── ssh.rs                # SSH connection management
│   │   ├── db.rs                 # Database operations
│   │   ├── models.rs             # Data structures
│   │   └── tunnel.rs             # SSH tunnel management
│   ├── Cargo.toml                # Rust dependencies
│   └── tauri.conf.json           # Tauri configuration
├── package.json                  # Node.js dependencies
├── tsconfig.json                 # TypeScript configuration
├── tailwind.config.js            # TailwindCSS configuration
└── vite.config.ts                # Vite build configuration
```

---

## Quick Start

### Prerequisites

Ensure your development environment has:

- **Node.js** (v16 or higher) - [Download](https://nodejs.org/)
- **Rust toolchain** - [Install Guide](https://www.rust-lang.org/tools/install)
- **Git** - [Download](https://git-scm.com/)

### Installation

1. Clone the repository

   ```bash
   git clone https://github.com/yourusername/ssh-ssistant-tauri.git
   cd ssh-ssistant-tauri
   ```

2. Install dependencies

   ```bash
   npm install
   ```

### Development

Start the development server with hot reload:

```bash
npm run tauri dev
```

This command will:

- Start the Vite frontend development server
- Open the Tauri application window
- Enable hot reload for both frontend and backend changes

### Build for Production

Build the release application:

```bash
npm run tauri build
```

The built application will be located in:

- **Windows**: `src-tauri/target/release/bundle/nsis/`
- **macOS**: `src-tauri/target/release/bundle/dmg/`
- **Linux**: `src-tauri/target/release/bundle/appimage/`

### Frontend Only Development

To work only on the frontend:

```bash
npm run dev
```

### Type Checking

Check TypeScript types without building:

```bash
npm run build
```

---

## Usage Guide

### Creating an SSH Connection

1. Click the "New Connection" button
2. Fill in the connection details:

   - **Name**: Connection identifier
   - **Host**: Server address or IP
   - **Port**: SSH port (default: 22)
   - **Username**: SSH username
   - **Password**: Authentication password
   - **Jump Host** (optional): Bastion server details

3. Click "Save" to store the connection

### Managing Sessions

- **Open Session**: Double-click on a saved connection
- **Switch Sessions**: Click on different tabs
- **Close Session**: Click the × button on the tab

### File Management

1. Navigate to the "Files" tab in any session
2. **Upload Files**: Drag and drop files into the file list
3. **Download Files**: Right-click → "Download"
4. **Edit Files**: Right-click → "Edit" to open in local editor

### SSH Tunnels

1. Go to "Tunnels" in the session menu
2. Click "Add Tunnel"
3. Configure:

   - **Type**: Local/Remote/Dynamic
   - **Local Port**: Port on your machine
   - **Remote Host**: Target server
   - **Remote Port**: Target port

4. Click "Create" to establish the tunnel

### AI Assistant

1. Open the "AI Assistant" panel
2. Configure your AI API in Settings
3. Start typing commands or questions
4. Execute suggested commands directly from the chat

---

## Configuration

### AI Configuration

Configure in Settings → AI Assistant:

- **API Endpoint**: Your AI service URL
- **API Key**: Authentication key
- **Model Name**: AI model to use
- **Temperature**: Response randomness (0-1)

### Terminal Settings

- **Font Size**: Adjust terminal text size
- **Cursor Style**: Block, underline, or bar
- **Scrollback Lines**: Terminal history size
- **Copy on Select**: Auto-copy selected text

### File Transfer Settings

- **Buffer Size**: Transfer buffer (KB)
- **Max Retries**: Retry attempts for failed transfers
- **Timeout**: Connection timeout (seconds)

---

## Contributing

We welcome contributions! Please follow these steps:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Development Guidelines

- Follow the existing code style
- Add tests for new features
- Update documentation as needed
- Ensure TypeScript type safety
- Test on multiple platforms when possible

---

## FAQ

### Q: Does SSH Assistant support SSH key authentication?

A: Yes, you can import and manage SSH keys in the SSH Key Manager.

### Q: Can I use multiple AI providers?

A: Currently, we support any OpenAI-compatible API endpoint. You can configure different providers by changing the API endpoint in settings.

### Q: Is my connection data secure?

A: All connections are stored locally in an SQLite database. Passwords are encrypted. We never transmit your data to external servers.

### Q: Can I transfer files between two remote servers?

A: Currently, file transfers are between your local machine and remote servers. Direct server-to-server transfer is planned for a future release.

### Q: How do I enable debug logging?

A: Set the `RUST_LOG=debug` environment variable before running the application.

---

## Known Issues

- ZModem transfers may not work on all terminal emulators
- Some terminal color schemes may not render correctly in the integrated terminal
- Large file uploads (>2GB) may require increasing the buffer size in settings

See [GitHub Issues](https://github.com/yourusername/ssh-ssistant-tauri/issues) for the complete list.

---

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

## Acknowledgments

- [Tauri](https://tauri.app/) - Amazing framework for building desktop applications
- [Vue.js](https://vuejs.org/) - The progressive JavaScript framework
- [xterm.js](https://xtermjs.org/) - Excellent terminal emulator for the web
- [ssh2-rs](https://github.com/alexcrichton/ssh2-rs) - Rust SSH client library
- All contributors and users of SSH Assistant

---

## Support & Community

- **GitHub**: <https://github.com/yourusername/ssh-ssistant-tauri>
- **Issues**: <https://github.com/yourusername/ssh-ssistant-tauri/issues>
- **Discussions**: <https://github.com/yourusername/ssh-ssistant-tauri/discussions>

---

**Built with ❤️ using Tauri + Vue 3**

[⬆ Back to Top](#ssh-star)
