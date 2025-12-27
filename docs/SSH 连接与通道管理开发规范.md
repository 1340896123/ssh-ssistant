# SSH 连接与通道管理开发规范 (V1.0)

## 1. 核心设计原则 (Core Principles)

* **多路复用优先 (Multiplexing First)**：严禁为每一个操作（如执行一个命令）创建一个新的 TCP/SSH 连接。必须利用 SSH 协议的多路复用特性，在单个 `Connection` 上并发创建多个 `Channel`。
* **资源隔离 (Isolation)**：不同的业务上下文（如：终端 Tab、文件传输、后台心跳）应使用独立的 `Channel`，互不干扰。
* **生命周期明确 (Explicit Lifecycle)**：必须严格管理 Connection 和 Channel 的 Open、EOF、Close 状态，防止句柄泄漏。

---

## 2. 连接层 (Connection) 管理规范

SSH Connection 指底层的 TCP 连接及加密会话。它是昂贵的资源（握手耗时、加密开销大）。

### 2.1 连接池与单例模式
* **规范**：对于同一目标主机（Host + Port + User），在应用程序生命周期内应维护**单例（Singleton）**或**连接池**。
* **反例**：用户点击“刷新文件列表”时，重新建立一个 SSH 连接。
* **正例**：检查现有连接是否活跃 (`isActive`)。如果是，复用之；如果否，重连。

### 2.2 心跳保活 (Keep-Alive)
* **规范**：必须在应用层配置心跳，防止防火墙或 NAT 设备因连接空闲而切断 TCP。
* **参数建议**：
    * `ServerAliveInterval`: 15-30秒 (客户端向服务器发送空包的时间间隔)。
    * `ServerAliveCountMax`: 3 (超过次数未响应则判定断开)。
* **实现**：大多数 SSH 库（如 Java JSch, Rust ssh2/russh）均提供此配置，不要手动开启线程发 ping。

### 2.3 重连机制 (Reconnection)
* **规范**：网络抖动导致连接断开时，UI 层面应给予用户反馈，自动重连应遵循**指数退避 (Exponential Backoff)** 策略。
* **策略**：第1次重试等待1s，第2次2s，第3次4s... 直至上限。

---

## 3. 通道层 (Channel) 使用规范

Channel 是轻量级的，创建开销极小。根据业务场景选择正确的 Channel 类型。

### 3.1 场景一：交互式终端 (Terminal Shell)
* **Channel 类型**：`session`
* **必须操作**：
    1.  Request PTY (伪终端分配)：指定终端类型（如 `xterm-256color`）。
    2.  Start Shell：执行 shell 请求。
    3.  **Window Change Handling**：必须监听前端终端组件的大小变化事件，并实时向 Channel 发送 `SIGWINCH` (Window Dimension Change) 信号，否则 `vim` 或 `top` 会排版混乱。
* **生命周期**：与前端 Tab 页绑定。Tab 关闭 -> 发送 EOF -> 关闭 Channel。

### 3.2 场景二：单次命令执行 (One-off Command)
* **Channel 类型**：`session` -> `exec`
* **规范**：
    * **不要使用 Shell Channel** 执行非交互命令（如 `ls`, `cat`）。这会导致你需要去解析提示符（Prompt），极易出错。
    * 应使用 `exec` 通道。发送命令 -> 读取 Stdout/Stderr -> 接收 Exit Code -> 销毁 Channel。
* **典型用途**：获取服务器系统信息、获取目录列表（如果不走 SFTP）。

### 3.3 场景三：文件传输 (SFTP)
* **Channel 类型**：`subsystem` (name: `sftp`)
* **并发策略**：
    * **浏览与轻量操作**：建议复用**1个**常驻的 SFTP Channel。
    * **大文件传输**：建议为耗时的大文件上传/下载创建**临时且独立**的 SFTP Channel。
    * *原因*：SFTP 是基于请求-响应的。如果一个 Channel 正在阻塞写入大文件，同一个 Channel 上的 `ls` 请求会被阻塞。
* **缓冲区**：设置合理的 Buffer Size（通常 32KB - 64KB）以最大化吞吐量。

### 3.4 场景四：端口转发 (Port Forwarding)
* **Channel 类型**：`direct-tcpip` (本地转发) 或 `forwarded-tcpip` (远程转发)。
* **管理**：端口转发的 Channel 是**按需动态创建**的。
    * 当浏览器访问本地映射端口时，SSH 客户端会收到请求，此时才创建 Channel。
    * TCP 连接断开，Channel 随之关闭。

---

## 4. 并发限制与流控 (Concurrency & Flow Control)

服务器端通常对并发 Channel 数量有限制（`sshd_config` 中的 `MaxSessions`，默认为 10）。

### 4.1 客户端限流
* **规范**：客户端应维护一个信号量（Semaphore）或计数器，限制单连接下的最大并发 Channel 数（建议设置为 10 以内，或通过配置暴露给用户）。
* **处理拒绝**：如果创建 Channel 失败（服务器返回拒绝），必须捕获异常，并提示用户“连接数过多，请关闭部分标签页”。

### 4.2 数据流控 (Windowing)
* **规范**：如果在开发底层 SSH 处理（如使用 Rust 直接操作），必须尊崇 SSH 协议的 Window 机制。
* **操作**：
    * 当收到 `SSH_MSG_CHANNEL_WINDOW_ADJUST` 时，才能继续发送数据。
    * 不要无脑向 Channel 灌入数据，否则会导致内存溢出或连接被服务器强制重置。

---

## 5. 异常处理与资源释放 (Cleanup)

### 5.1 优雅关闭流程 (Graceful Shutdown)
永远不要直接销毁对象，应遵循协议握手：
1.  **Input Shutdown**：发送 EOF (End of File)，告知对端“我没有数据发了”。
2.  **Wait**：等待对端也发送 EOF（可选，视超时而定）。
3.  **Close**：发送 Close 消息。
4.  **Disconnect**：如果是关闭整个程序，最后断开 TCP 连接。

### 5.2 僵尸 Channel 检测
* **现象**：网络异常断开后，本地对象可能还认为 Channel 是 Open 的。
* **规范**：捕获所有 IO 异常。一旦 Connection 级别的 IO 报错，必须标记该 Connection 下**所有** Channel 为失效，并触发清理逻辑。

---

## 6. 开发速查表 (Cheat Sheet)

| 功能场景 | 动作序列 | Channel 生命周期 | 是否独占 Channel? |
| :--- | :--- | :--- | :--- |
| **打开终端 Tab** | Open Session -> Request PTY -> Shell | 长期 (直到关闭 Tab) | 是 |
| **获取 CPU 使用率** | Open Session -> Exec "top -b -n 1" | 短期 (毫秒级) | 是 (用完即毁) |
| **SFTP 浏览目录** | Open Subsystem "sftp" -> Send Packet | 长期 (直到断开) | 建议复用 |
| **上传 1GB 文件** | Open Subsystem "sftp" -> Write Loop | 中长期 (直到传完) | **建议新建独立 Channel** |
| **本地端口转发** | Open direct-tcpip | 随 TCP 请求动态创建 | 是 |

---

