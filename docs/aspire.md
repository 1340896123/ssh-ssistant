# Aspire 本地编排

SSH Assistant 使用 .NET Aspire 同时管理管理 API 与 Vue 后台网站。

## 资源拓扑

- `admin-api`：`backend/SshAssistant.AdminApi`，提供管理接口、SQLite 数据与后台任务。
- `admin-web`：`admin`，由 Vite 提供开发服务器，并通过 `/api` 代理访问 `admin-api`。
- Aspire Dashboard：统一展示资源状态、结构化日志、Trace、Metrics 与健康检查。

## 启动

在仓库根目录运行：

```powershell
npm run dev:admin
```

也可以使用已安装的 Aspire CLI：

```powershell
aspire run
```

启动后浏览器会打开 Aspire Dashboard。进入 `admin-web` 资源即可打开后台网站。

默认后台账号：

- 用户名：`admin`
- 密码：`admin123`

## 独立开发

不通过 Aspire 时，可以分别启动 API 和管理站：

```powershell
dotnet run --project backend/SshAssistant.AdminApi/SshAssistant.AdminApi.csproj --launch-profile http
npm --prefix admin run dev
```

Vite 会将 `/api` 请求代理到 `http://localhost:5047`。

## 构建

```powershell
npm run build:admin
```

Aspire 发布模型会构建 `admin`，并将产物装入管理 API 的 `wwwroot`；API 使用 SPA fallback 支持后台路由直接访问。
