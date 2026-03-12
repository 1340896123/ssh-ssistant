# 数据库迁移方案 - file_manager_layout 字段

## 需求概述

**目标**: 在 `settings` 表中添加 `file_manager_layout` 字段，实现文件管理器布局配置的持久化存储。

**业务需求**:
- 用户可以在设置中切换文件管理器的布局模式（左侧/底部）
- 布局选择需要在应用重启后保持
- 提供向后兼容性，现有用户平滑升级

---

## 一、数据库设计

### 1.1 字段定义

| 属性 | 值 |
|------|-----|
| **字段名** | `file_manager_layout` |
| **数据类型** | TEXT |
| **约束** | NOT NULL |
| **默认值** | 'bottom' |
| **有效值** | 'left' \| 'bottom' |

### 1.2 业务含义

| 值 | 说明 | UI 表现 |
|----|------|---------|
| `'left'` | 左侧布局 | 文件管理器显示在终端左侧，分左右两栏 |
| `'bottom'` | 底部布局 | 文件管理器显示在终端下方，分上下两栏 |

### 1.3 为什么选择 TEXT 而非 INTEGER

**理由**:
1. **可读性**: TEXT 值 'left'/'bottom' 比 0/1 更直观，便于数据库调试
2. **扩展性**: 未来可能添加新的布局模式（如 'right', 'top', 'floating'）
3. **一致性**: 与现有 `file_manager_view_mode` 字段 ('flat'/'tree') 保持一致
4. **类型安全**: Rust 和 TypeScript 都使用强类型枚举，运行时验证值的有效性

---

## 二、数据库迁移 SQL

### 2.1 迁移语句

```sql
-- 在 init_db() 函数中添加以下迁移
ALTER TABLE settings ADD COLUMN file_manager_layout TEXT NOT NULL DEFAULT 'bottom';
```

### 2.2 代码位置

**文件**: `d:\source\ssh-ssistant\src-tauri\src\db.rs`
**函数**: `init_db()`
**插入位置**: 在第 99 行（file_manager_sftp_buffer_size 迁移）之后

**完整代码片段**:
```rust
// Migration: Add SFTP buffer size
let _ = conn.execute(
    r#"ALTER TABLE settings ADD COLUMN file_manager_sftp_buffer_size INTEGER NOT NULL DEFAULT 512"#,
    [],
);

// Migration: Add file manager layout (NEW)
let _ = conn.execute(
    r#"ALTER TABLE settings ADD COLUMN file_manager_layout TEXT NOT NULL DEFAULT 'bottom'"#,
    [],
);
```

### 2.3 迁移安全性分析

✅ **安全特性**:
1. **幂等性**: SQLite 的 `ALTER TABLE ADD COLUMN` 如果字段已存在会报错，但使用 `let _ =` 忽略错误，不影响已迁移的数据库
2. **默认值**: 现有用户升级时自动填充默认值 'bottom'
3. **NOT NULL**: 确保数据完整性，不会有 NULL 值
4. **无数据丢失**: 仅添加新字段，不修改或删除现有数据

---

## 三、数据模型更新

### 3.1 Rust 后端模型

**文件**: `d:\source\ssh-ssistant\src-tauri\src\models.rs`

**当前代码** (第 74-79 行):
```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FileManagerSettings {
    pub view_mode: String,
    pub sftp_buffer_size: i32,
}
```

**需要修改为**:
```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FileManagerSettings {
    pub view_mode: String,
    pub layout: String,        // 新增字段
    pub sftp_buffer_size: i32,
}
```

### 3.2 TypeScript 前端类型

**文件**: `d:\source\ssh-ssistant\src\types.ts`

**当前代码** (第 61-68 行):
```typescript
export type FileManagerViewMode = "flat" | "tree";
export type FileManagerLayout = "left" | "bottom";

export interface FileManagerSettings {
  viewMode: FileManagerViewMode;
  layout: FileManagerLayout;
  sftpBufferSize: number;
}
```

**状态**: ✅ 已正确定义，无需修改

### 3.3 前端 Store 默认值

**文件**: `d:\source\ssh-ssistant\src\stores\settings.ts`

**当前代码** (第 21-25 行):
```typescript
fileManager: {
  viewMode: 'flat',
  layout: 'bottom',
  sftpBufferSize: 512
},
```

**状态**: ✅ 已正确定义，无需修改

---

## 四、CRUD 操作更新

### 4.1 get_settings() 查询更新

**文件**: `d:\source\ssh-ssistant\src-tauri\src\db.rs`
**函数**: `get_settings()`
**行号**: 428-508

**当前 SQL 查询** (第 432 行):
```sql
SELECT theme, language, ai_api_url, ai_api_key, ai_model_name,
       terminal_font_size, terminal_font_family, terminal_cursor_style, terminal_line_height,
       file_manager_view_mode, ssh_max_background_sessions, ssh_enable_auto_cleanup,
       ssh_cleanup_interval_minutes, file_manager_sftp_buffer_size,
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
FROM settings WHERE id = 1
```

**需要添加**: 在 `file_manager_view_mode` 后添加 `, file_manager_layout`

**修改后查询**:
```sql
SELECT theme, language, ai_api_url, ai_api_key, ai_model_name,
       terminal_font_size, terminal_font_family, terminal_cursor_style, terminal_line_height,
       file_manager_view_mode, file_manager_layout, ssh_max_background_sessions, ssh_enable_auto_cleanup,
       -- ... 其余字段保持不变
```

**当前 Rust 解析代码** (第 455-460 行):
```rust
file_manager: FileManagerSettings {
    view_mode: row
        .get::<_, Option<String>>(9)?
        .unwrap_or_else(|| "flat".to_string()),
    sftp_buffer_size: row.get::<_, Option<i32>>(13)?.unwrap_or(512),
},
```

**需要修改为**:
```rust
file_manager: FileManagerSettings {
    view_mode: row
        .get::<_, Option<String>>(9)?
        .unwrap_or_else(|| "flat".to_string()),
    layout: row
        .get::<_, Option<String>>(10)?
        .unwrap_or_else(|| "bottom".to_string()),
    sftp_buffer_size: row.get::<_, Option<i32>>(14)?.unwrap_or(512),  // 注意索引变为 14
},
```

**索引调整**: 由于添加了新字段，后续所有字段的索引需要 +1

### 4.2 save_settings() 保存更新

**文件**: `d:\source\ssh-ssistant\src-tauri\src\db.rs`
**函数**: `save_settings()`
**行号**: 510-559

**当前 SQL 更新** (第 516 行):
```sql
UPDATE settings SET theme=?1, language=?2, ai_api_url=?3, ai_api_key=?4, ai_model_name=?5,
    terminal_font_size=?6, terminal_font_family=?7, terminal_cursor_style=?8,
    terminal_line_height=?9, file_manager_view_mode=?10,
    ssh_max_background_sessions=?11, ssh_enable_auto_cleanup=?12,
    ssh_cleanup_interval_minutes=?13, file_manager_sftp_buffer_size=?14,
    connection_timeout_secs=?15, jump_host_timeout_secs=?16,
    local_forward_timeout_secs=?17, command_timeout_secs=?18,
    sftp_operation_timeout_secs=?19, reconnect_max_attempts=?20,
    reconnect_initial_delay_ms=?21, reconnect_max_delay_ms=?22,
    reconnect_backoff_multiplier=?23, reconnect_enabled=?24,
    heartbeat_tcp_keepalive_interval_secs=?25,
    heartbeat_ssh_keepalive_interval_secs=?26,
    heartbeat_app_heartbeat_interval_secs=?27, heartbeat_timeout_secs=?28,
    heartbeat_failed_heartbeats_before_action=?29,
    pool_health_check_interval_secs=?30, pool_session_warmup_count=?31,
    pool_max_session_age_minutes=?32, pool_unhealthy_threshold=?33,
    network_adaptive_enabled=?34, network_latency_check_interval_secs=?35,
    network_high_latency_threshold_ms=?36, network_low_bandwidth_threshold_kbps=?37
WHERE id = 1
```

**需要添加**: 在 `file_manager_view_mode=?10` 后添加 `, file_manager_layout=?11`

**修改后更新**:
```sql
UPDATE settings SET theme=?1, language=?2, ai_api_url=?3, ai_api_key=?4, ai_model_name=?5,
    terminal_font_size=?6, terminal_font_family=?7, terminal_cursor_style=?8,
    terminal_line_height=?9, file_manager_view_mode=?10, file_manager_layout=?11,
    ssh_max_background_sessions=?12, ssh_enable_auto_cleanup=?13,
    -- ... 所有后续参数索引 +1
```

**当前 Rust 参数传递** (第 517-556 行):
```rust
params![
    settings.theme,
    settings.language,
    settings.ai.api_url,
    settings.ai.api_key,
    settings.ai.model_name,
    settings.terminal_appearance.font_size,
    settings.terminal_appearance.font_family,
    settings.terminal_appearance.cursor_style,
    settings.terminal_appearance.line_height,
    settings.file_manager.view_mode,
    settings.ssh_pool.max_background_sessions,
    // ... 其余参数
]
```

**需要添加**: 在 `settings.file_manager.view_mode` 后添加 `settings.file_manager.layout`

**修改后参数**:
```rust
params![
    settings.theme,
    settings.language,
    settings.ai.api_url,
    settings.ai.api_key,
    settings.ai.model_name,
    settings.terminal_appearance.font_size,
    settings.terminal_appearance.font_family,
    settings.terminal_appearance.cursor_style,
    settings.terminal_appearance.line_height,
    settings.file_manager.view_mode,
    settings.file_manager.layout,  // 新增
    settings.ssh_pool.max_background_sessions,
    // ... 其余参数保持不变
]
```

---

## 五、影响分析报告

### 5.1 受影响的文件清单

| 文件路径 | 操作 | 说明 |
|----------|------|------|
| `src-tauri/src/db.rs` | 修改 | 添加迁移 SQL，更新 CRUD 操作 |
| `src-tauri/src/models.rs` | 修改 | FileManagerSettings 添加 layout 字段 |
| `src/types.ts` | 无需修改 | 类型定义已存在 |
| `src/stores/settings.ts` | 无需修改 | 默认值已存在 |

### 5.2 代码变更统计

| 语言 | 文件数 | 新增行 | 修改行 | 删除行 |
|------|--------|--------|--------|--------|
| Rust | 2 | ~3 | ~15 | 0 |
| TypeScript | 0 | 0 | 0 | 0 |
| SQL | 1 | 1 | 2 | 0 |
| **总计** | **3** | **~4** | **~17** | **0** |

### 5.3 风险评估

| 风险类型 | 等级 | 说明 | 缓解措施 |
|----------|------|------|----------|
| 数据迁移失败 | 🟢 低 | 使用标准 ALTER TABLE 语法 | 幂等性设计，错误忽略 |
| 索引错位 | 🟡 中 | 查询和保存的索引需要调整 | 仔细核对索引顺序 |
| 类型不匹配 | 🟢 低 | TEXT 类型与 Rust String 完美兼容 | serde 自动序列化 |
| 向后兼容性 | 🟢 低 | 新字段有默认值 | 现有用户无感知升级 |

### 5.4 测试计划

#### 单元测试
1. **数据库迁移测试**
   - 验证新数据库创建时字段存在
   - 验证旧数据库升级时字段添加成功
   - 验证默认值正确填充

2. **CRUD 操作测试**
   - 验证 `get_settings()` 正确读取 layout 值
   - 验证 `save_settings()` 正确保存 layout 值
   - 验证数据往返一致性（保存→读取→对比）

#### 集成测试
1. **前后端数据同步**
   - 前端切换布局 → 后端保存 → 重启应用 → 验证布局保持
   - 前端默认值与后端默认值一致性

2. **UI 交互测试**
   - 设置页面布局切换功能
   - 文件管理器布局渲染
   - 跨会话布局状态保持

#### 回归测试
- 确保现有配置项不受影响
- 确保数据库性能无明显下降

---

## 六、实施步骤

### Phase 1: 数据库层 (Rust)
**文件**: `src-tauri/src/db.rs`

1. **添加迁移** (在第 99 行后):
   ```rust
   // Migration: Add file manager layout
   let _ = conn.execute(
       r#"ALTER TABLE settings ADD COLUMN file_manager_layout TEXT NOT NULL DEFAULT 'bottom'"#,
       [],
   );
   ```

2. **更新 get_settings() 查询** (第 432 行):
   - 在 SELECT 语句中添加 `file_manager_layout`
   - 调整后续字段索引

3. **更新 get_settings() 解析** (第 455 行):
   - 添加 `layout` 字段解析
   - 调整 `sftp_buffer_size` 索引

4. **更新 save_settings() SQL** (第 516 行):
   - 在 UPDATE 语句中添加 `file_manager_layout=?11`
   - 调整后续参数索引

5. **更新 save_settings() 参数** (第 527 行):
   - 在 params! 宏中添加 `settings.file_manager.layout`

### Phase 2: 数据模型 (Rust)
**文件**: `src-tauri/src/models.rs`

1. **更新 FileManagerSettings 结构体** (第 76 行):
   ```rust
   pub struct FileManagerSettings {
       pub view_mode: String,
       pub layout: String,  // 新增
       pub sftp_buffer_size: i32,
   }
   ```

### Phase 3: 编译验证
```bash
cd d:\source\ssh-ssistant-tauri
npm run build  # 触发 TypeScript 类型检查
cd src-tauri
cargo check    # 验证 Rust 代码编译
```

### Phase 4: 功能测试
```bash
npm run tauri dev
```

**测试用例**:
1. 启动应用，检查默认布局是否为 'bottom'
2. 打开设置，切换布局为 'left'
3. 重启应用，验证布局保持为 'left'
4. 切换回 'bottom'，验证保存成功

---

## 七、验证清单

### 开发完成验证
- [ ] 数据库迁移 SQL 已添加
- [ ] Rust 模型已更新
- [ ] get_settings() 查询已更新
- [ ] get_settings() 解析已更新
- [ ] save_settings() SQL 已更新
- [ ] save_settings() 参数已更新

### 编译验证
- [ ] TypeScript 编译无错误 (`npm run build`)
- [ ] Rust 编译无错误 (`cargo check`)
- [ ] 无类型警告或弃用警告

### 功能验证
- [ ] 新用户首次启动，默认值为 'bottom'
- [ ] 老用户升级，自动填充默认值 'bottom'
- [ ] 切换布局后保存成功
- [ ] 重启应用布局保持
- [ ] 设置页面 UI 正常显示布局选项

### 数据一致性验证
- [ ] 前端默认值与后端默认值一致
- [ ] 数据库存储值与前端显示值一致
- [ ] Rust 序列化与 TypeScript 反序列化一致

---

## 八、回滚方案

如果实施后发现问题，回滚步骤：

### 方案 A: 代码回滚
```bash
git checkout HEAD -- src-tauri/src/db.rs src-tauri/src/models.rs
```

### 方案 B: 数据库回滚（慎用）
```sql
-- 仅在开发环境使用，生产环境不推荐
ALTER TABLE settings DROP COLUMN file_manager_layout;
```

**注意**: SQLite 的 `DROP COLUMN` 在某些版本可能不支持，建议使用代码回滚并忽略该字段。

---

## 九、性能影响评估

| 指标 | 影响评估 | 说明 |
|------|----------|------|
| 查询性能 | 🟢 无影响 | 单行查询，添加一个 TEXT 字段可忽略 |
| 存储空间 | 🟢 可忽略 | 单行数据，TEXT 类型约 5-10 字节 |
| 序列化性能 | 🟢 无影响 | serde 对小字符串优化良好 |
| 应用启动时间 | 🟢 无影响 | settings 表加载在毫秒级 |

---

## 十、后续优化建议

1. **迁移版本管理** (可选)
   - 考虑添加 `schema_version` 表记录当前迁移版本
   - 便于未来复杂迁移的管理和回滚

2. **数据验证** (可选)
   - 在 Rust 模型层添加 layout 值的验证
   - 确保只能是 'left' 或 'bottom'

3. **迁移日志** (可选)
   - 记录迁移执行时间和结果
   - 便于问题排查

---

**文档版本**: v1.0
**创建日期**: 2026-03-10
**最后更新**: 2026-03-10
**作者**: 数据库架构师
**审核状态**: 待系统架构师审核
**实施状态**: 待开发
