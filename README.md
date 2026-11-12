# MQTTXX

MQTT 3.1.1 / 5.0 桌面客户端 —— 基于 **Tauri 2 + Vue 3 + Rust** 对 [MQTTX](./MQTTX-main) 的完全重构。

## 为什么重构

原版 MQTTX（Electron + Vue 2）存在几个结构性性能瓶颈，本重构全部针对性地解决：

| 原版瓶颈 | 本重构方案 |
|---|---|
| mqtt.js 跑在渲染进程，消息洪峰与 UI 抢主线程 | **rumqttc 在 Rust 侧 tokio 任务中运行**，UI 崩溃/刷新不影响连接 |
| TypeORM 逐条写库 | 消息进缓冲区，**攒批（≥100 条或 150ms）事务写入 SQLite** |
| 桌面版消息列表无虚拟滚动 | 手写 **VirtualList 定高虚拟滚动**，仅渲染可视区 ± buffer |
| Electron 内存 200MB+、包体 100MB+ | Tauri 2 系统级 WebView，前端产物 **gzip 53 KB** |
| Vue 2（EOL）巨型组件 | Vue 3 `<script setup>` + Pinia，组件全部轻量化 |

## 架构

```
┌─ Tauri 2 ─────────────────────────────────────────────────┐
│  WebView（Vue 3 + Pinia + vue-router）                     │
│   · ConnectionSidebar / ConnectionForm / Subscriptions    │
│   · MessageList（VirtualList 虚拟滚动） / PublishBar       │
│   · 事件订阅：conn-status / messages                       │
│        ▲ invoke() commands        │ emit() events ▼       │
│  Rust 核心（tokio）                                        │
│   ├─ mqtt_manager: 每连接一个 EventLoop 任务               │
│   │    · rumqttc 0.24（MQTT 3.1.1 + 5.0，TLS/rustls）     │
│   │    · mpsc 命令通道（publish/subscribe/disconnect）     │
│   │    · 消息批处理 → 事务落库 + events 批量推送前端        │
│   ├─ db: rusqlite（bundled SQLite，WAL，PRAGMA 迁移）      │
│   │    表：connections / subscriptions / messages / settings│
│   └─ settings: KV 配置                                     │
└───────────────────────────────────────────────────────────┘
```

### 关键设计

- **连接生命周期在 Rust 侧**：断网时 rumqttc 自动重连；前端重连成功后按订阅列表自动恢复订阅（`auto_resub`）。
- **消息洪峰不阻塞**：Rust 缓冲区攒批 → 单事务写入 → 一次性 emit，前端只做一次数组追加 + 虚拟渲染。
- **TLS**：默认 rustls（证书校验）；支持粘贴 CA 证书（PEM）；自签名场景可勾选「跳过证书校验」（自定义 `ServerCertVerifier` 全接受）。
- **数据目录**：`%APPDATA%/com.mqttxx.desktop/mqttxx.db`（SQLite，WAL 模式，`user_version` 迁移）。

## 开发

```bash
npm install          # 安装前端依赖
npm run tauri dev    # 开发模式（Vite HMR + Rust 自动重编）
npm run tauri build  # 发布构建（Windows 下产出 NSIS + MSI 安装包）
npm run build        # 仅构建前端（vue-tsc 类型检查 + vite build）
```

要求：Node ≥ 18、Rust stable（MSVC）、Windows 上需 VS Build Tools（链接器）。

## 发布与自动更新

1. 升版本：改 `src-tauri/tauri.conf.json` 的 `version`（客户端以此与线上版本比较）
2. 打包：`npm run tauri build`，产物在 `src-tauri/target/release/bundle/`（`nsis/…-setup.exe`、`msi/….msi`）
3. 发版：到 Gitee 仓库 `oslu/mqttxx` → **版本发布（Releases）** → 新建 Release：
   - 标签填 `v<版本号>`（如 `v0.2.0`，客户端自动去掉 v 前缀比较）
   - 正文写更新日志（会显示在客户端更新卡片里）
   - 上传安装包附件（`-setup.exe` 或 `.msi`）
4. 客户端：启动时自动检查（可在设置关闭），或 设置 → 检查更新；下载完成点"立即安装"，应用退出并静默安装，重开即新版本

## 当前范围（v0.1）

已实现（对标 MQTTX 核心功能）：

- ✅ 多连接管理（增删改查、搜索、右键菜单、连接重入自动断开重连）
- ✅ **服务器地址协议选择器**：`mqtt://` / `mqtts://` / `ws://` / `wss://`（端口随协议自动切换，ws/wss 支持自定义 Path）
- ✅ MQTT 3.1.1 / 5.0、用户名密码、Keep Alive、Clean Session、连接超时
- ✅ **SSL/TLS 三证书（文件方式添加）**：CA 证书 + 客户端证书 + 客户端私钥（双向认证，适配阿里云等 EMQX 云场景）；未选 CA 时回退系统根证书；自签名可跳过校验
- ✅ **遗嘱消息（Will）**：主题 / 内容 / QoS / Retain
- ✅ 连接/断开/自动重连/状态可视化（connecting / connected / reconnecting / error）
- ✅ 订阅管理（QoS 0/1/2、主题色、持久化、连接后自动恢复、在线增删订阅）
- ✅ 消息发布（QoS、Retain、JSON 格式化、Ctrl+Enter）
- ✅ 消息列表（虚拟滚动、收发方向、主题过滤、历史分页加载、搜索、清空）
- ✅ 消息详情（JSON pretty / raw 切换、复制、二进制 Base64 展示）
- ✅ 全部消息 SQLite 持久化 + 分页回看；DB 结构化迁移（v1→v2 已在线验证）
- ✅ 设置（浅色/深色主题、中英文、内存消息上限、默认自动重订阅）
- ✅ 集成测试：阿里云三证书解析 + 真实 broker 五种传输（v4/v5/TLS/WS/WSS）回环全通过

后续路线图（原版对应模块，按需补齐）：

- ⬜ MQTT 5 属性面板（session expiry / receive maximum 等）
- ⬜ Script 脚本（Rust 侧嵌 QuickJS 沙箱替代 vm2）
- ⬜ protobuf/avro/cbor 编解码
- ⬜ 主题树 / Dashboard / 流量监控
- ⬜ AI Copilot（HTTP 直连 + MCP，Rust `rmcp` SDK）
- ⬜ 连接配置导入导出、密码/证书 OS 级加密（当前明文存 SQLite）

## 目录结构

```
├── index.html / vite.config.ts / tsconfig.json   # 前端工程
├── src/                      # Vue 3 前端
│   ├── api.ts                # invoke 封装 + 事件监听
│   ├── i18n.ts               # 轻量中英文案
│   ├── stores/               # Pinia（app 设置 / connections 运行态）
│   ├── components/           # VirtualList、侧栏、表单、消息列表等
│   └── views/                # WorkspaceView / SettingsView
├── scripts/gen-icon.mjs      # 生成应用图标（纯 Node，无依赖）
└── src-tauri/                # Rust 核心
    ├── tauri.conf.json
    ├── capabilities/default.json
    └── src/
        ├── lib.rs            # 入口：setup + 命令注册
        ├── mqtt_manager.rs   # 连接管理器（核心）
        ├── commands.rs       # 15 个 Tauri 命令
        ├── db.rs             # SQLite + 模型 + 查询
        └── settings.rs       # KV 设置
```

License: Apache-2.0（与 MQTTX 一致）
