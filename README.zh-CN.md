# SSH Assistant (SSH 远程连接助手)

中文 | **[English](README.md)**

基于 Tauri + Vue 3 + TypeScript 构建的现代化 SSH 客户端。它集成了强大的终端功能、AI 智能助手以及全功能文件管理器，旨在提升远程服务器管理效率。

![示例图片](exampleImg/image.png)

## ✨ 功能特性

- **连接管理**：

  - 支持创建、编辑和删除 SSH 连接。
  - 支持密码认证方式。
  - 支持跳板机（Jump Host/Bastion）连接。
  - 使用 SQLite 本地持久化存储连接信息。

- **会话管理**：

  - 多标签页支持，同时管理多个服务器会话。
  - 会话间完全隔离，互不干扰。
  - 每个会话集成终端、文件管理和 AI 助手。

- **智能终端**：

  - 基于 xterm.js 的全功能终端。
  - 支持传统 Tab 自动补全。
  - **AI 智能补全**：基于上下文的智能命令建议。

- **文件管理**：

  - 远程文件浏览、上传、下载。
  - 支持拖拽上传和下载。
  - 文件/目录的创建、删除、重命名。
  - 本地编辑远程文件（自动下载 -> 监控修改 -> 自动上传）。
  - 断点续传与文件完整性校验。

- **AI 助手**：

  - 上下文感知的智能对话。
  - 直接在聊天界面执行终端命令。
  - 可配置 AI 模型参数（API 地址、密钥、模型名称）。

- **个性化配置**：
  - 多语言支持（中文/英文）。
  - 界面主题切换。
  - 自定义 AI 补全和对话配置。

## 🛠️ 技术栈

- **核心框架**：[Tauri v2](https://tauri.app/) (Rust)
- **前端框架**：[Vue 3](https://vuejs.org/) + [TypeScript](https://www.typescriptlang.org/)
- **构建工具**：[Vite](https://vitejs.dev/)
- **样式库**：[TailwindCSS](https://tailwindcss.com/)
- **状态管理**：[Pinia](https://pinia.vuejs.org/)
- **终端组件**：[xterm.js](https://xtermjs.org/)
- **图标库**：[Lucide Vue](https://lucide.dev/)

## 🚀 快速开始

### 环境要求

确保您的开发环境已安装：

- [Node.js](https://nodejs.org/) (v16+)
- [Rust](https://www.rust-lang.org/tools/install) (用于 Tauri 后端编译)

### 安装依赖

1.  克隆项目：

    ```bash
    git clone <repository-url>
    cd ssh-ssistant
    ```

2.  安装 NPM 依赖：
    ```bash
    npm install
    ```

### 开发模式

启动开发服务器（支持热重载）：

```bash
npm run tauri dev
```

该命令将启动 Vite 前端服务并打开 Tauri 应用窗口。

### 构建发布

构建生产环境应用：

```bash
npm run tauri build
```

构建产物将生成在 `src-tauri/target/release/` 目录下。

## 📂 项目结构

- `src/`: 前端 Vue 源码
  - `components/`: Vue 组件 (Terminal, FileManager, AIAssistant 等)
  - `stores/`: Pinia 状态管理
  - `i18n/`: 国际化资源文件
- `src-tauri/`: 后端 Rust 源码
  - `src/`: SSH 连接、文件操作及系统交互实现
  - `tauri.conf.json`: Tauri 配置文件

## 📝 开源协议

[MIT](LICENSE)
