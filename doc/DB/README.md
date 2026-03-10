# 数据库架构文档 - 快速导航

## 📚 文档索引

本文档目录包含 SSH Assistant 项目的所有数据库设计文档。

### 核心文档

| 文档 | 说明 | 维护者 |
|------|------|--------|
| [settings.md](./settings.md) | settings 表完整设计文档 | 数据库架构师 |
| [layout_field_migration_plan.md](./layout_field_migration_plan.md) | layout 字段迁移实施计划 | 数据库架构师 |
| [database_impact_analysis.md](./database_impact_analysis.md) | 数据库变更影响分析报告 | 数据库架构师 |

---

## 🗄️ 数据库表总览

SSH Assistant 使用 SQLite 数据库，包含以下表：

### 主表结构

```
ssh_assistant.db
├── connections              - SSH 连接配置
├── connection_groups        - 连接分组
├── ssh_keys                - SSH 密钥管理
├── transfer_records        - 文件传输记录
└── settings                - 应用配置 (单例模式)
```

### settings 表字段分类

#### 1. 核心配置 (3 字段)
- theme, language, (id)

#### 2. AI 配置 (3 字段)
- ai_api_url, ai_api_key, ai_model_name

#### 3. 终端外观 (4 字段)
- terminal_font_size, terminal_font_family
- terminal_cursor_style, terminal_line_height

#### 4. 文件管理器 (3 字段) ⭐
- file_manager_view_mode
- **file_manager_layout** (新增)
- file_manager_sftp_buffer_size

#### 5. SSH 连接池 (3 字段)
- ssh_max_background_sessions
- ssh_enable_auto_cleanup
- ssh_cleanup_interval_minutes

#### 6. 连接超时 (5 字段)
- connection_timeout_secs, jump_host_timeout_secs
- local_forward_timeout_secs, command_timeout_secs
- sftp_operation_timeout_secs

#### 7. 自动重连 (5 字段)
- reconnect_max_attempts, reconnect_initial_delay_ms
- reconnect_max_delay_ms, reconnect_backoff_multiplier
- reconnect_enabled

#### 8. 心跳检测 (5 字段)
- heartbeat_tcp_keepalive_interval_secs
- heartbeat_ssh_keepalive_interval_secs
- heartbeat_app_heartbeat_interval_secs
- heartbeat_timeout_secs
- heartbeat_failed_heartbeats_before_action

#### 9. 连接池健康 (4 字段)
- pool_health_check_interval_secs
- pool_session_warmup_count
- pool_max_session_age_minutes
- pool_unhealthy_threshold

#### 10. 网络自适应 (4 字段)
- network_adaptive_enabled
- network_latency_check_interval_secs
- network_high_latency_threshold_ms
- network_low_bandwidth_threshold_kbps

**总计**: 39 个字段

---

## 🔄 最新变更 (v2.0)

### 新增字段: file_manager_layout

**用途**: 支持文件管理器布局模式持久化

**字段定义**:
```sql
file_manager_layout TEXT NOT NULL DEFAULT 'bottom'
```

**有效值**:
- `'left'` - 左侧布局模式
- `'bottom'` - 底部布局模式

**相关文档**:
- [迁移方案](./layout_field_migration_plan.md)
- [影响分析](./database_impact_analysis.md)

---

## 📋 数据库规范

### 命名约定

| 类型 | 约定 | 示例 |
|------|------|------|
| **表名** | snake_case | `transfer_records` |
| **字段名** | snake_case | `file_manager_layout` |
| **Rust 结构体** | PascalCase | `FileManagerSettings` |
| **Rust 字段** | snake_case | `view_mode` |
| **TypeScript 接口** | PascalCase | `FileManagerSettings` |
| **TypeScript 字段** | camelCase | `viewMode` |

### 数据类型映射

| SQLite | Rust | TypeScript |
|--------|------|------------|
| TEXT | String | string |
| INTEGER | i32 / u32 | number |
| REAL | f32 / f64 | number |
| BLOB | Vec<u8> | string (base64) |

### 约束规范

- **主键**: 所有表都有 `id INTEGER PRIMARY KEY`
- **默认值**: 所有非主键字段都有 DEFAULT 值
- **NOT NULL**: 所有字段都设置为 NOT NULL
- **外键**: 关联字段使用 REFERENCES 约束
- **级联**: 重要关联使用 ON DELETE CASCADE

---

## 🔧 维护指南

### 添加新字段的标准流程

1. **设计阶段**
   - [ ] 确定字段类型和约束
   - [ ] 定义默认值
   - [ ] 评估向后兼容性

2. **后端实现 (Rust)**
   - [ ] 在 `models.rs` 添加字段定义
   - [ ] 在 `db.rs` 的 `init_db()` 添加迁移 SQL
   - [ ] 更新 `get_settings()` 查询
   - [ ] 更新 `save_settings()` 保存
   - [ ] 更新相关索引

3. **前端实现 (TypeScript)**
   - [ ] 在 `types.ts` 添加类型定义
   - [ ] 在 `stores/settings.ts` 设置默认值
   - [ ] 更新 UI 组件（如需要）

4. **测试验证**
   - [ ] 编译测试 (`cargo check` + `npm run build`)
   - [ ] 单元测试
   - [ ] 集成测试
   - [ ] 手动功能测试

5. **文档更新**
   - [ ] 更新表设计文档
   - [ ] 记录迁移版本
   - [ ] 更新字段分类

### 迁移版本记录

| 版本 | 日期 | 变更说明 |
|------|------|----------|
| v1.0 | 初始版本 | 基础 settings 表 |
| v1.1 | - | 终端外观字段 |
| v1.2 | - | 文件管理器视图模式 |
| v1.3 | - | SSH 连接池配置 |
| v1.4 | - | SFTP 缓冲区大小 |
| v1.5 | - | 连接超时配置 |
| v1.6 | - | 自动重连配置 |
| v1.7 | - | 心跳检测配置 |
| v1.8 | - | 连接池健康配置 |
| v1.9 | - | 网络自适应配置 |
| v2.0 | 2026-03-10 | **文件管理器布局字段** |

---

## 📍 数据库位置

### 开发环境
```
C:\Users\jieok\AppData\Roaming\com.ssh-assistant.app\ssh_assistant.db
```

### 生产环境
```
%APPDATA%\com.ssh-assistant.app\ssh_assistant.db
```

### 跨平台路径
- **Windows**: `%APPDATA%\com.ssh-assistant.app\ssh_assistant.db`
- **macOS**: `~/Library/Application Support/com.ssh-assistant.app/ssh_assistant.db`
- **Linux**: `~/.config/com.ssh-assistant.app/ssh_assistant.db`

---

## 🛠️ 常用操作

### 查看数据库结构
```bash
sqlite3 ssh_assistant.db ".schema settings"
```

### 查询当前设置
```bash
sqlite3 ssh_assistant.db "SELECT * FROM settings"
```

### 手动添加 layout 字段（开发测试）
```bash
sqlite3 ssh_assistant.db "ALTER TABLE settings ADD COLUMN file_manager_layout TEXT NOT NULL DEFAULT 'bottom'"
```

### 验证字段存在
```bash
sqlite3 ssh_assistant.db "PRAGMA table_info(settings)"
```

---

## 📞 联系方式

**数据库架构师**: AI Agent
**创建日期**: 2026-03-10
**最后更新**: 2026-03-10
**文档版本**: v1.0

---

## 📝 变更日志

### 2026-03-10
- 创建数据库文档目录
- 完成 settings 表设计文档
- 完成 layout 字段迁移方案
- 完成数据库影响分析报告
- 创建本导航文档
