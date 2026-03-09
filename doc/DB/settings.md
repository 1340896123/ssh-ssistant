# Settings 表数据库设计文档

## 表概述

**表名**: `settings`
**用途**: 存储应用程序的全局配置和用户偏好设置
**数据库**: SQLite (`ssh_assistant.db`)
**位置**: `C:\Users\jieok\AppData\Roaming\com.ssh-assistant.app\ssh_assistant.db`

## 表结构

### 设计约束
- **单例模式**: 该表只允许一行数据 (id = 1)
- **主键**: `id INTEGER PRIMARY KEY CHECK (id = 1)` 确保只能有一行配置

### 字段定义

#### 核心配置字段

| 字段名 | 类型 | 约束 | 默认值 | 说明 |
|--------|------|------|--------|------|
| `id` | INTEGER | PRIMARY KEY CHECK (id = 1) | 1 | 单例标识符，确保只有一行 |
| `theme` | TEXT | NOT NULL | 'dark' | UI主题：'dark' 或 'light' |
| `language` | TEXT | NOT NULL | 'zh' | 界面语言：'zh' 或 'en' |

#### AI 配置字段

| 字段名 | 类型 | 约束 | 默认值 | 说明 |
|--------|------|------|--------|------|
| `ai_api_url` | TEXT | NOT NULL | 'https://api.openai.com/v1' | AI API 端点 URL |
| `ai_api_key` | TEXT | NOT NULL | '' | AI API 密钥 |
| `ai_model_name` | TEXT | NOT NULL | 'gpt-3.5-turbo' | AI 模型名称 |

#### 终端外观配置字段

| 字段名 | 类型 | 约束 | 默认值 | 说明 |
|--------|------|------|--------|------|
| `terminal_font_size` | INTEGER | NOT NULL | 14 | 终端字体大小 (像素) |
| `terminal_font_family` | TEXT | NOT NULL | 'Menlo, Monaco, "Courier New", monospace' | 终端字体家族 |
| `terminal_cursor_style` | TEXT | NOT NULL | 'block' | 光标样式：'block', 'underline', 'bar' |
| `terminal_line_height` | REAL | NOT NULL | 1.0 | 行高倍数 |

#### 文件管理器配置字段

| 字段名 | 类型 | 约束 | 默认值 | 说明 |
|--------|------|------|--------|------|
| `file_manager_view_mode` | TEXT | NOT NULL | 'flat' | 文件浏览模式：'flat' 或 'tree' |
| `file_manager_layout` | TEXT | NOT NULL | 'bottom' | **文件管理器布局：'left' 或 'bottom'** |
| `file_manager_sftp_buffer_size` | INTEGER | NOT NULL | 512 | SFTP 缓冲区大小 (KB) |

#### SSH 连接池配置字段

| 字段名 | 类型 | 约束 | 默认值 | 说明 |
|--------|------|------|--------|------|
| `ssh_max_background_sessions` | INTEGER | NOT NULL | 10 | 最大后台会话数 |
| `ssh_enable_auto_cleanup` | INTEGER | NOT NULL | 1 | 启用自动清理 (0=否, 1=是) |
| `ssh_cleanup_interval_minutes` | INTEGER | NOT NULL | 5 | 清理间隔 (分钟) |

#### 连接超时配置字段

| 字段名 | 类型 | 约束 | 默认值 | 说明 |
|--------|------|------|--------|------|
| `connection_timeout_secs` | INTEGER | NOT NULL | 15 | 连接超时 (秒) |
| `jump_host_timeout_secs` | INTEGER | NOT NULL | 30 | 跳板机超时 (秒) |
| `local_forward_timeout_secs` | INTEGER | NOT NULL | 10 | 本地转发超时 (秒) |
| `command_timeout_secs` | INTEGER | NOT NULL | 30 | 命令执行超时 (秒) |
| `sftp_operation_timeout_secs` | INTEGER | NOT NULL | 60 | SFTP 操作超时 (秒) |

#### 自动重连配置字段

| 字段名 | 类型 | 约束 | 默认值 | 说明 |
|--------|------|------|--------|------|
| `reconnect_max_attempts` | INTEGER | NOT NULL | 5 | 最大重连尝试次数 |
| `reconnect_initial_delay_ms` | INTEGER | NOT NULL | 1000 | 初始重连延迟 (毫秒) |
| `reconnect_max_delay_ms` | INTEGER | NOT NULL | 30000 | 最大重连延迟 (毫秒) |
| `reconnect_backoff_multiplier` | REAL | NOT NULL | 2.0 | 退避倍数 |
| `reconnect_enabled` | INTEGER | NOT NULL | 1 | 启用自动重连 (0=否, 1=是) |

#### 心跳检测配置字段

| 字段名 | 类型 | 约束 | 默认值 | 说明 |
|--------|------|------|--------|------|
| `heartbeat_tcp_keepalive_interval_secs` | INTEGER | NOT NULL | 60 | TCP keepalive 间隔 (秒) |
| `heartbeat_ssh_keepalive_interval_secs` | INTEGER | NOT NULL | 15 | SSH keepalive 间隔 (秒) |
| `heartbeat_app_heartbeat_interval_secs` | INTEGER | NOT NULL | 30 | 应用层心跳间隔 (秒) |
| `heartbeat_timeout_secs` | INTEGER | NOT NULL | 5 | 心跳超时 (秒) |
| `heartbeat_failed_heartbeats_before_action` | INTEGER | NOT NULL | 3 | 触发操作的失败心跳数 |

#### 连接池健康配置字段

| 字段名 | 类型 | 约束 | 默认值 | 说明 |
|--------|------|------|--------|------|
| `pool_health_check_interval_secs` | INTEGER | NOT NULL | 60 | 健康检查间隔 (秒) |
| `pool_session_warmup_count` | INTEGER | NOT NULL | 1 | 预热会话数量 |
| `pool_max_session_age_minutes` | INTEGER | NOT NULL | 60 | 会话最大存活时间 (分钟) |
| `pool_unhealthy_threshold` | INTEGER | NOT NULL | 3 | 判定为不健康的失败次数 |

#### 网络自适应配置字段

| 字段名 | 类型 | 约束 | 默认值 | 说明 |
|--------|------|------|--------|------|
| `network_adaptive_enabled` | INTEGER | NOT NULL | 1 | 启用自适应 (0=否, 1=是) |
| `network_latency_check_interval_secs` | INTEGER | NOT NULL | 30 | 延迟检测间隔 (秒) |
| `network_high_latency_threshold_ms` | INTEGER | NOT NULL | 300 | 高延迟阈值 (毫秒) |
| `network_low_bandwidth_threshold_kbps` | INTEGER | NOT NULL | 100 | 低带宽阈值 (Kbps) |

## 数据库迁移历史

### 初始表结构 (v1.0)
```sql
CREATE TABLE IF NOT EXISTS settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    theme TEXT NOT NULL DEFAULT 'dark',
    language TEXT NOT NULL DEFAULT 'zh',
    ai_api_url TEXT NOT NULL DEFAULT 'https://api.openai.com/v1',
    ai_api_key TEXT NOT NULL DEFAULT '',
    ai_model_name TEXT NOT NULL DEFAULT 'gpt-3.5-turbo',
    terminal_font_size INTEGER NOT NULL DEFAULT 14,
    terminal_font_family TEXT NOT NULL DEFAULT 'Menlo, Monaco, "Courier New", monospace',
    terminal_cursor_style TEXT NOT NULL DEFAULT 'block',
    terminal_line_height REAL NOT NULL DEFAULT 1.0
)
```

### 迁移记录

#### v1.1 - 终端外观字段 (已集成到初始表)
- 添加 `terminal_font_size`
- 添加 `terminal_font_family`
- 添加 `terminal_cursor_style`
- 添加 `terminal_line_height`

#### v1.2 - 文件管理器视图模式
```sql
ALTER TABLE settings ADD COLUMN file_manager_view_mode TEXT NOT NULL DEFAULT 'flat'
```

#### v1.3 - SSH 连接池配置
```sql
ALTER TABLE settings ADD COLUMN ssh_max_background_sessions INTEGER NOT NULL DEFAULT 10
ALTER TABLE settings ADD COLUMN ssh_enable_auto_cleanup INTEGER NOT NULL DEFAULT 1
ALTER TABLE settings ADD COLUMN ssh_cleanup_interval_minutes INTEGER NOT NULL DEFAULT 5
```

#### v1.4 - SFTP 缓冲区大小
```sql
ALTER TABLE settings ADD COLUMN file_manager_sftp_buffer_size INTEGER NOT NULL DEFAULT 512
```

#### v1.5 - 连接超时配置
```sql
ALTER TABLE settings ADD COLUMN connection_timeout_secs INTEGER NOT NULL DEFAULT 15
ALTER TABLE settings ADD COLUMN jump_host_timeout_secs INTEGER NOT NULL DEFAULT 30
ALTER TABLE settings ADD COLUMN local_forward_timeout_secs INTEGER NOT NULL DEFAULT 10
ALTER TABLE settings ADD COLUMN command_timeout_secs INTEGER NOT NULL DEFAULT 30
ALTER TABLE settings ADD COLUMN sftp_operation_timeout_secs INTEGER NOT NULL DEFAULT 60
```

#### v1.6 - 自动重连配置
```sql
ALTER TABLE settings ADD COLUMN reconnect_max_attempts INTEGER NOT NULL DEFAULT 5
ALTER TABLE settings ADD COLUMN reconnect_initial_delay_ms INTEGER NOT NULL DEFAULT 1000
ALTER TABLE settings ADD COLUMN reconnect_max_delay_ms INTEGER NOT NULL DEFAULT 30000
ALTER TABLE settings ADD COLUMN reconnect_backoff_multiplier REAL NOT NULL DEFAULT 2.0
ALTER TABLE settings ADD COLUMN reconnect_enabled INTEGER NOT NULL DEFAULT 1
```

#### v1.7 - 心跳检测配置
```sql
ALTER TABLE settings ADD COLUMN heartbeat_tcp_keepalive_interval_secs INTEGER NOT NULL DEFAULT 60
ALTER TABLE settings ADD COLUMN heartbeat_ssh_keepalive_interval_secs INTEGER NOT NULL DEFAULT 15
ALTER TABLE settings ADD COLUMN heartbeat_app_heartbeat_interval_secs INTEGER NOT NULL DEFAULT 30
ALTER TABLE settings ADD COLUMN heartbeat_timeout_secs INTEGER NOT NULL DEFAULT 5
ALTER TABLE settings ADD COLUMN heartbeat_failed_heartbeats_before_action INTEGER NOT NULL DEFAULT 3
```

#### v1.8 - 连接池健康配置
```sql
ALTER TABLE settings ADD COLUMN pool_health_check_interval_secs INTEGER NOT NULL DEFAULT 60
ALTER TABLE settings ADD COLUMN pool_session_warmup_count INTEGER NOT NULL DEFAULT 1
ALTER TABLE settings ADD COLUMN pool_max_session_age_minutes INTEGER NOT NULL DEFAULT 60
ALTER TABLE settings ADD COLUMN pool_unhealthy_threshold INTEGER NOT NULL DEFAULT 3
```

#### v1.9 - 网络自适应配置
```sql
ALTER TABLE settings ADD COLUMN network_adaptive_enabled INTEGER NOT NULL DEFAULT 1
ALTER TABLE settings ADD COLUMN network_latency_check_interval_secs INTEGER NOT NULL DEFAULT 30
ALTER TABLE settings ADD COLUMN network_high_latency_threshold_ms INTEGER NOT NULL DEFAULT 300
ALTER TABLE settings ADD COLUMN network_low_bandwidth_threshold_kbps INTEGER NOT NULL DEFAULT 100
```

#### v2.0 - **文件管理器布局字段 (新增)**
```sql
ALTER TABLE settings ADD COLUMN file_manager_layout TEXT NOT NULL DEFAULT 'bottom'
```

**业务含义**:
- `'left'`: 左侧布局模式 - 文件管理器显示在终端左侧
- `'bottom'`: 底部布局模式 - 文件管理器显示在终端下方

## CRUD 操作

### 读取配置 (get_settings)
**SQL 查询**:
```sql
SELECT theme, language, ai_api_url, ai_api_key, ai_model_name,
       terminal_font_size, terminal_font_family, terminal_cursor_style, terminal_line_height,
       file_manager_view_mode, file_manager_layout, file_manager_sftp_buffer_size,
       ssh_max_background_sessions, ssh_enable_auto_cleanup, ssh_cleanup_interval_minutes,
       connection_timeout_secs, jump_host_timeout_secs, local_forward_timeout_secs,
       command_timeout_secs, sftp_operation_timeout_secs,
       reconnect_max_attempts, reconnect_initial_delay_ms, reconnect_max_delay_ms,
       reconnect_backoff_multiplier, reconnect_enabled,
       heartbeat_tcp_keepalive_interval_secs, heartbeat_ssh_keepalive_interval_secs,
       heartbeat_app_heartbeat_interval_secs, heartbeat_timeout_secs,
       heartbeat_failed_heartbeats_before_action,
       pool_health_check_interval_secs, pool_session_warmup_count,
       pool_max_session_age_minutes, pool_unhealthy_threshold,
       network_adaptive_enabled, network_latency_check_interval_secs,
       network_high_latency_threshold_ms, network_low_bandwidth_threshold_kbps
FROM settings
WHERE id = 1
```

### 保存配置 (save_settings)
**SQL 更新**:
```sql
UPDATE settings
SET theme = ?1, language = ?2, ai_api_url = ?3, ai_api_key = ?4, ai_model_name = ?5,
    terminal_font_size = ?6, terminal_font_family = ?7, terminal_cursor_style = ?8,
    terminal_line_height = ?9, file_manager_view_mode = ?10,
    ssh_max_background_sessions = ?11, ssh_enable_auto_cleanup = ?12,
    ssh_cleanup_interval_minutes = ?13, file_manager_sftp_buffer_size = ?14,
    connection_timeout_secs = ?15, jump_host_timeout_secs = ?16,
    local_forward_timeout_secs = ?17, command_timeout_secs = ?18,
    sftp_operation_timeout_secs = ?19, reconnect_max_attempts = ?20,
    reconnect_initial_delay_ms = ?21, reconnect_max_delay_ms = ?22,
    reconnect_backoff_multiplier = ?23, reconnect_enabled = ?24,
    heartbeat_tcp_keepalive_interval_secs = ?25,
    heartbeat_ssh_keepalive_interval_secs = ?26,
    heartbeat_app_heartbeat_interval_secs = ?27, heartbeat_timeout_secs = ?28,
    heartbeat_failed_heartbeats_before_action = ?29,
    pool_health_check_interval_secs = ?30, pool_session_warmup_count = ?31,
    pool_max_session_age_minutes = ?32, pool_unhealthy_threshold = ?33,
    network_adaptive_enabled = ?34, network_latency_check_interval_secs = ?35,
    network_high_latency_threshold_ms = ?36, network_low_bandwidth_threshold_kbps = ?37
WHERE id = 1
```

**注意**: 更新语句需要添加 `file_manager_layout` 字段

## 数据类型映射

### Rust → SQLite → TypeScript

| Rust 类型 | SQLite 类型 | TypeScript 类型 | 示例值 |
|-----------|-------------|-----------------|--------|
| `String` | TEXT | `string` | 'dark', 'left' |
| `i32` | INTEGER | `number` | 14, 512 |
| `u32` | INTEGER | `number` | 15, 60 |
| `f32` | REAL | `number` | 1.0, 2.0 |
| `bool` | INTEGER (0/1) | `boolean` | true/false |

## 前后端数据同步

### Frontend (TypeScript)
**类型定义** (`src/types.ts`):
```typescript
export type FileManagerLayout = "left" | "bottom";

export interface FileManagerSettings {
  viewMode: FileManagerViewMode;
  layout: FileManagerLayout;      // 新增字段
  sftpBufferSize: number;
}
```

**默认值** (`src/stores/settings.ts`):
```typescript
fileManager: {
  viewMode: 'flat',
  layout: 'bottom',      // 前端默认值
  sftpBufferSize: 512
}
```

### Backend (Rust)
**类型定义** (`src-tauri/src/models.rs`):
```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FileManagerSettings {
    pub view_mode: String,
    pub sftp_buffer_size: i32,
    // TODO: 添加 pub layout: String,
}
```

**需要更新**: `FileManagerSettings` 结构体需要添加 `layout` 字段

## 索引设计

无需索引 - settings 表为单例模式，仅一行数据

## 数据完整性约束

### CHECK 约束
- `id CHECK (id = 1)` - 确保只能有一行配置数据

### NOT NULL 约束
- 所有字段都设置为 `NOT NULL`，确保没有未定义的配置

### 默认值策略
- 所有字段都有合理的默认值
- 新字段添加时使用 `ALTER TABLE ... ADD COLUMN ... DEFAULT value` 语法
- 已有用户的数据库会自动填充默认值

## 性能优化

### 查询优化
- 使用 `WHERE id = 1` 直接定位唯一行，无需索引
- 单例模式避免了多行查询的开销

### 缓存策略
- 前端使用 Pinia store 缓存配置
- 配置变更时通过 `save_settings` 立即持久化
- 应用启动时通过 `loadSettings()` 加载配置

## 版本兼容性

### 向后兼容性
- ✅ 所有迁移使用 `ALTER TABLE ... ADD COLUMN ... IF NOT EXISTS` 模式
- ✅ 新字段都有默认值，现有用户无感知升级
- ✅ 布尔值使用 INTEGER (0/1) 存储兼容旧版 SQLite

### 字段命名约定
- ✅ 使用 snake_case (Rust/SQLite 约定)
- ✅ 前端通过 serde `rename_all = "camelCase"` 自动转换
- ✅ 保持命名一致性：`file_manager_*` 前缀用于文件管理器相关字段

## 相关表关系

settings 表是独立配置表，不与其他表建立外键关系。

### 数据库表结构总览
```
ssh_assistant.db
├── connections          - SSH 连接配置
├── connection_groups    - 连接分组
├── ssh_keys            - SSH 密钥管理
├── transfer_records    - 文件传输记录
└── settings            - 应用配置 (本文档) ← 单例模式
```

## 维护说明

### 添加新配置字段的标准流程
1. **Rust 模型**: 在 `src-tauri/src/models.rs` 添加字段定义
2. **数据库迁移**: 在 `init_db()` 函数添加 `ALTER TABLE` 语句
3. **查询更新**: 更新 `get_settings()` 的 SELECT 语句
4. **保存更新**: 更新 `save_settings()` 的 UPDATE 语句
5. **类型定义**: 在 `src/types.ts` 添加 TypeScript 类型
6. **默认值**: 在 `src/stores/settings.ts` 设置默认值
7. **文档更新**: 更新本文档

---

**文档版本**: v2.0
**创建日期**: 2026-03-10
**最后更新**: 2026-03-10
**维护者**: 数据库架构师
**状态**: 已完成 layout 字段设计，待实施
