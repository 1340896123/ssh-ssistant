# SSH Assistant

基于 Tauri 2、Vue 3、TypeScript 和 Rust 构建的现代化桌面 SSH 客户端，面向开发者、运维和 DevOps 场景，提供终端、远程文件管理、SSH 隧道和 AI 助手的一体化工作台。

[![Tauri](https://img.shields.io/badge/Tauri-2-FFC131?logo=tauri&logoColor=white)](https://tauri.app/)
[![Vue](https://img.shields.io/badge/Vue-3-4FC08D?logo=vue.js&logoColor=white)](https://vuejs.org/)
[![TypeScript](https://img.shields.io/badge/TypeScript-5-3178C6?logo=typescript&logoColor=white)](https://www.typescriptlang.org/)
[![Rust](https://img.shields.io/badge/Rust-2021-000000?logo=rust)](https://www.rust-lang.org/)

![SSH Assistant Screenshot](exampleImg/image.png)

## 项目概述

SSH Assistant 是一个将原生桌面能力与现代前端交互体验结合起来的 SSH 工具。

它的目标不是只提供一个终端窗口，而是围绕“连接服务器后的完整工作流”来设计：

- 管理和保存 SSH 连接
- 同时打开多个会话标签页
- 直接浏览、上传、下载和编辑远程文件
- 创建和维护 SSH 隧道
- 在会话上下文中使用 AI 助手辅助命令执行
- 持久化本地设置、连接信息和密钥数据

## 核心能力

### 1. SSH 连接与会话管理

- 支持创建、编辑、删除和测试连接
- 支持跳板机/堡垒机场景
- 支持多标签页并行管理多个服务器会话
- 会话状态、连接数据和用户配置持久化保存到本地 SQLite

### 2. 集成终端

- 基于 `xterm.js` 提供终端体验
- 支持终端写入、窗口尺寸调整、搜索和链接识别
- 可结合 AI 助手提供命令建议和辅助执行

### 3. 远程文件管理

- 基于 SFTP 浏览远程目录和文件
- 支持上传、下载、重命名、删除、创建文件/目录
- 支持传输进度、暂停、恢复、失败重试和记录清理
- 支持将远程文件拉到本地编辑后再回写上传

### 4. SSH 隧道

- 支持本地端口转发
- 支持远程端口转发
- 支持动态端口转发（SOCKS）
- 支持查看当前活动隧道并进行启停管理

### 5. AI 助手

- 在会话上下文中提供问答与命令辅助
- 支持配置 API 地址、模型名称和相关参数
- 可结合终端内容和文件路径提供更贴近上下文的建议

### 6. 系统信息与辅助能力

- 支持获取远程系统状态、磁盘使用情况和服务器状态
- 支持 SSH 密钥生成、管理和安装
- 支持中英文界面
- 支持本地通知、设置项和布局持久化

## 技术栈

### 前端

- `Vue 3` + Composition API
- `TypeScript`
- `Vite`
- `Pinia`
- `TailwindCSS`
- `xterm.js`
- `Monaco Editor`
- `vue-i18n`

### 后端

- `Tauri 2`
- `Rust 2021`
- `tokio`
- `ssh2`
- `rusqlite`
- `serde`

## 项目结构

```text
ssh-ssistant-tauri/
├── src/                    # Vue 3 前端
│   ├── components/         # 终端、文件管理、连接列表、AI 助手等组件
│   ├── stores/             # Pinia 状态管理
│   ├── composables/        # 通用组合式逻辑
│   ├── i18n/               # 国际化资源
│   └── App.vue             # 应用主界面
├── src-tauri/              # Tauri + Rust 后端
│   ├── src/
│   │   ├── lib.rs          # Tauri 命令注册与应用初始化
│   │   ├── db.rs           # SQLite 数据持久化
│   │   ├── models.rs       # 数据模型
│   │   ├── system.rs       # 系统辅助能力
│   │   └── ssh/            # SSH、SFTP、终端、隧道、传输等核心逻辑
│   └── tauri.conf.json     # Tauri 配置
├── public/                 # 静态资源
├── doc/ docs/              # 设计文档与补充资料
└── README.md               # 项目说明
```

## 开发环境

- Node.js 16+
- Rust stable toolchain
- npm

安装依赖：

```bash
npm install
```

## 开发与构建

启动前端开发服务器：

```bash
npm run dev
```

启动 Tauri 桌面开发模式：

```bash
npm run tauri dev
```

构建前端：

```bash
npm run build
```

构建桌面应用：

```bash
npm run tauri build
```

## 常用脚本

| 命令 | 说明 |
| --- | --- |
| `npm run dev` | 启动前端开发服务器 |
| `npm run build` | 执行类型检查并构建前端 |
| `npm run tauri dev` | 启动 Tauri 开发模式 |
| `npm run tauri build` | 构建桌面应用 |
| `npm run release:patch` | 发布补丁版本 |
| `npm run release:minor` | 发布次版本 |
| `npm run release:major` | 发布主版本 |

## 适用场景

- 日常 SSH 登录与多服务器管理
- 远程文件上传、下载和编辑
- 通过隧道暴露本地或远程服务
- 将 AI 能力接入服务器运维和命令执行流程
- 需要桌面端本地持久化和原生能力支持的 SSH 工作台

## 说明

- 连接数据、设置和部分业务数据保存在本地 SQLite 中
- AI 能力依赖用户自行配置的模型服务接口
- 项目采用前后端分离式桌面架构：前端负责交互，Rust 后端负责 SSH、文件传输和系统能力
