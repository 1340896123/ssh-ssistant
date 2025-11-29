# 发布流程

## 自动发布 (推荐)

### 方式1: GitHub Actions 自动触发

当推送带有 `v` 前缀的 tag 时，GitHub Actions 会自动：
1. 构建多平台应用 (Windows, macOS, Linux)
2. 创建 GitHub Release
3. 上传编译好的包

```bash
# 创建并推送 tag
git tag v1.5.0
git push origin v1.5.0
```

### 方式2: 本地发布脚本

使用 npm 脚本一键发布：

```bash
# 补丁版本 (1.5.0 -> 1.5.1)
npm run release:patch

# 次要版本 (1.5.0 -> 1.6.0)  
npm run release:minor

# 主要版本 (1.5.0 -> 2.0.0)
npm run release:major

# 手动发布当前版本
npm run release
```

## 手动发布

### 方式1: GitHub Actions 手动触发

1. 进入 GitHub 仓库的 Actions 页面
2. 选择 "Manual Release" 工作流
3. 点击 "Run workflow"
4. 输入版本号 (如: 1.5.0)
5. 选择是否创建 git tag
6. 点击 "Run workflow"

### 方式2: 本地构建 + 手动上传

```bash
# 构建应用
npm run tauri build

# 找到编译好的包
# Windows: src-tauri/target/release/bundle/msi/ 和 nsis/
# macOS: src-tauri/target/release/bundle/macos/
# Linux: src-tauri/target/release/bundle/deb/ 和 rpm/
```

然后在 GitHub 页面手动：
1. 创建 Release
2. 上传编译好的包文件

## 文件位置

编译好的包位于：
- **Windows MSI**: `src-tauri/target/release/bundle/msi/SshStar_[version]_x64_en-US.msi`
- **Windows NSIS**: `src-tauri/target/release/bundle/nsis/SshStar_[version]_x64-setup.exe`
- **macOS DMG**: `src-tauri/target/release/bundle/macos/SshStar_[version]_x64.dmg`
- **Linux DEB**: `src-tauri/target/release/bundle/deb/ssh-star_[version]_amd64.deb`
- **Linux RPM**: `src-tauri/target/release/bundle/rpm/ssh-star-[version]-1.x86_64.rpm`

## 版本管理

项目使用统一的版本管理：
- 只需修改 `package.json` 中的版本号
- `tauri.conf.json` 和 `Cargo.toml` 会自动同步

```bash
# 设置特定版本
npm version 1.6.0 --no-git-tag-version
```
