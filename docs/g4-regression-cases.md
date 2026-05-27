# G4 回归用例覆盖

## 目标范围

- G4.1 个人账号回归用例通过
- G4.2 企业子账号回归用例通过
- G4.3 本地模式回归用例通过
- G4.4 三种模式互不串数据
- G4.5 回归用例有执行记录

## 自动化入口

- 云侧自动回归脚本：`npm run verify:g4`
- 输出结果：
  - `tmp/regression/g4-regression-latest.json`
  - `tmp/regression/g4-regression-latest.md`

## 用例清单

### G4.1 个人账号回归

- 前置条件：
  - Admin API 可访问。
  - 后台存在至少 1 个 `ownerType=personal` 的资产种子。
  - 可创建 personal 测试账号与 personal 订阅。
- 操作步骤：
  - 登录 personal 账号。
  - 拉取云端资产。
  - 修改并同步个人设置。
  - 读取 AI runtime 与 subscription snapshot。
- 预期结果：
  - 可登录。
  - 可拉取个人资产。
  - 设置可保存且回拉一致。
  - AI runtime 可用。
  - 订阅与账单归属为 `personal`。
- 自动化覆盖：
  - `scripts/verify-g4-regression.mjs`
  - `node scripts/verify-g4-ai-call.mjs`

### G4.2 企业子账号回归

- 前置条件：
  - Admin API 可访问。
  - 后台存在至少 2 个 `ownerType=enterprise` 的资产种子。
  - 可创建 enterprise、sub-account 与 enterprise subscription。
- 操作步骤：
  - 登录企业子账号。
  - 拉取云端资产。
  - 修改并同步设置。
  - 读取 AI runtime 与 subscription snapshot。
- 预期结果：
  - 可登录。
  - 仅能看到授权资产。
  - 设置同步成功。
  - AI runtime 可用。
  - 订阅与账单归属为 `enterprise`。
- 自动化覆盖：
  - `scripts/verify-g4-regression.mjs`
  - `node scripts/verify-g4-ai-call.mjs`

### G4.3 本地模式回归

- 前置条件：
  - 以 Tauri 桌面态启动应用。
  - 本地 sqlite 可写。
  - 预置一份本地资产与设置样本。
- 操作步骤：
  - 在不登录云端的情况下进入 `local` 模式。
  - 校验可见本地工作区、资产列表、设置页、AI 自定义端点配置。
  - 修改本地设置并保存。
  - 从 cloud 模式切换回 `local`，验证本地快照恢复。
- 预期结果：
  - 可进入本地模式。
  - 本地核心功能可用。
  - 不会错误触发企业或个人云同步。
  - 切回本地模式后恢复本地快照。
- 当前覆盖方式：
  - Rust 自动化验证本地工作区快照 round-trip。
  - 人工桌面回归验证真实 UI 切换与交互。
- 自动化覆盖：
  - `cargo test local_workspace_snapshot_round_trip_restores_local_mode_without_cloud_residue --manifest-path src-tauri/Cargo.toml`
  - `node scripts/verify-g4-mode-transition.mjs`
  - `node scripts/verify-g4-web-switch.mjs`
  - `powershell -File scripts/verify-g4-native-local-switch.ps1`
- 关键代码参考：
  - `src/stores/settings.ts`
  - `src/components/SettingsModal.vue`
  - `src/App.vue`
  - `src/services/workspaceSnapshotService.ts`
  - `src-tauri/src/db.rs`

### G4.4 模式隔离回归

- 前置条件：
  - 已准备 personal、enterpriseSubAccount、local 三套独立样本。
- 操作步骤：
  - 自动验证 personal 与 enterprise 云侧隔离。
  - 在桌面态先进入 local，保存本地工作区快照。
  - 切换到 personal 或 enterprise，执行登录、同步。
  - 再切回 local，检查资产、设置、订阅、AI 配置是否恢复本地值。
- 预期结果：
  - personal 与 enterprise 不串资产、不串设置、不串订阅信息。
  - local 与 cloud 模式之间不串资产、不串设置、不串 AI 配置。
- 当前覆盖方式：
  - 云侧隔离自动化。
  - Rust 自动化验证 local 快照恢复不残留 cloud 数据。
  - 历史桌面态页面结构快照可佐证 local / personal 状态栏身份与订阅文案不同。
  - local 快照与前端状态恢复人工复验。
- 自动化覆盖：
  - `npm run verify:g4`
  - `cargo test local_workspace_snapshot_round_trip_restores_local_mode_without_cloud_residue --manifest-path src-tauri/Cargo.toml`
  - `node scripts/verify-g4-mode-transition.mjs`
  - `node scripts/verify-g4-web-switch.mjs`
  - `powershell -File scripts/verify-g4-native-local-switch.ps1`
- 现有桌面态证据：
  - `.playwright-cli/page-2026-05-25T16-28-05-800Z.yml`：状态栏显示 `Local Workspace · local`、`Free · http://localhost:5047`
  - `.playwright-cli/page-2026-05-25T16-30-12-350Z.yml`：状态栏显示 `Personal Demo · personal`、`Personal Pro · http://localhost:5047`
- 当前轮页面级证据：
  - `.playwright-cli/g4-web-current.png`：当前轮 Web 工作台实跑页面，状态栏显示 `Local Workspace · local`、`Free · local-only`
  - `.playwright-cli/g4-web-switch-flow.png`：当前轮点击 `Switch` 后进入登录网关页面
- 当前轮原生窗口链路证据：
  - `target/debug/app.exe` 当前轮已可单独启动，窗口标题为 `SshStar`
  - `.playwright-cli/g4-native-window-print.png`：当前轮原生窗口截图，至少可确认窗口成功创建
  - `.playwright-cli/g4-native-before-local-switch.png`：原生窗口登录网关起始态
  - `.playwright-cli/g4-native-local-workbench.png`：原生窗口切换到 local 工作台后的截图，状态栏显示 `Local Workspace · local`
  - `.playwright-cli/g4-native-back-to-gateway.png`：原生窗口点击 `Switch` 后切回登录网关

### G4.5 执行记录

- 前置条件：
  - 执行自动脚本或人工桌面回归。
- 操作步骤：
  - 输出 JSON/Markdown 回归结果。
  - 人工复验时继续补充实际结果与结论。
- 预期结果：
  - 每条用例均有测试前置、步骤、预期结果、实际结果和结论。
  - 失败项可挂缺陷单。
  - 通过项可复验。
- 自动化产物：
  - `tmp/regression/g4-regression-latest.json`
  - `tmp/regression/g4-regression-latest.md`
