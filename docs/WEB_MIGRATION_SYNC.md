# Web Migration 同步指南

本文档说明如何将上游仓库的更新同步到 Web 版本。

## 概述

Web 版本通过添加 Axum HTTP 服务器层替代 Tauri IPC 命令。核心游戏逻辑（`ofm_core`, `domain`, `engine`, `db` crates）保持不变。

---

## 改动分类

### 🔵 A 类 - 新增文件（低风险，易同步）

| 文件/目录 | 说明 | 同步方式 |
|-----------|------|----------|
| `src-tauri/src/http_server/` | HTTP 服务器模块 | 直接保留 |
| `src/lib/api.ts` | 前端 HTTP 适配器 | 直接保留 |
| `src/lib/tauri-polyfill.ts` | Tauri polyfill | 直接保留 |
| `src/lib/tauri.ts` | 重新导出 | 直接保留 |
| `postcss.config.js` | Tailwind v3 配置 | 直接保留 |
| `tailwind.config.js` | Tailwind v3 配置 | 直接保留 |
| `docs/WEB_VERSION.md` | 使用文档 | 直接保留 |

### 🟡 B 类 - 配置修改（低风险，轻微冲突）

| 文件 | 改动摘要 | 同步方式 |
|------|----------|----------|
| `package.json` | 添加 `dev:web` 脚本 | 手动合并 |
| `vite.config.ts` | 添加 web 模式配置、API 代理 | 手动合并 |
| `src-tauri/Cargo.toml` | 添加 axum, tokio 等依赖 | 手动合并 |

**同步步骤**：
1. 查看上游 `package.json` 最新脚本
2. 保留 `dev:web` 脚本，合并其他新增脚本
3. 同理处理 `vite.config.ts` 和 `Cargo.toml`

### 🟠 C 类 - 填充现有函数（低风险）

| 文件 | 函数 | 说明 |
|------|------|------|
| `src-tauri/src/http_server/handlers.rs` | `set_starting_xi` | 保存阵容 |
| `src-tauri/src/http_server/handlers.rs` | `set_team_match_roles` | 保存队长等角色 |
| `src-tauri/src/http_server/handlers.rs` | `set_formation` | 保存阵型 |
| `src-tauri/src/http_server/handlers.rs` | `set_play_style` | 保存比赛风格 |
| `src-tauri/src/http_server/handlers.rs` | `advance_time` | 添加自动放弃逻辑 |
| `src-tauri/src/http_server/handlers.rs` | `advance_time_with_mode` | 添加自动放弃逻辑 |

**同步步骤**：
1. 找到上游对应的函数（或新建）
2. 应用我们的改动逻辑
3. 保持其他参数处理不变

### 🔴 D 类 - 修改现有函数（中风险）

| 文件 | 函数 | 说明 |
|------|------|------|
| `src-tauri/src/lib.rs` | 添加 `run_web()` 和 web feature | 新增内容，几乎不冲突 |
| `src-tauri/src/main.rs` | 添加 CLI 解析 | 新增内容，几乎不冲突 |

---

## 详细改动说明

### 1. `src-tauri/src/http_server/handlers.rs` (1565 行)

这是最核心的文件，包含所有 HTTP API 处理程序。

#### 需要保留的 API 处理程序

| 函数名 | 行号 | 功能 |
|--------|------|------|
| `start_live_match` | ~470 | 创建直播比赛会话 |
| `step_live_match` | ~530 | 步进比赛模拟 |
| `get_match_snapshot` | ~560 | 获取比赛快照 |
| `apply_match_command` | ~580 | 应用比赛命令（换人等） |
| `finish_live_match` | ~620 | 完成比赛并生成报告 |
| `set_starting_xi` | ~680 | 保存首发阵容 |
| `set_team_match_roles` | ~710 | 保存比赛角色 |
| `set_formation` | ~658 | 保存阵型 |
| `set_play_style` | ~690 | 保存比赛风格 |
| `advance_time` | ~390 | 推进时间（需保留自动放弃逻辑） |
| `advance_time_with_mode` | ~425 | 带模式的推进时间 |

#### 关键逻辑（必须保留）

**自动放弃逻辑** - 在 `advance_time` 和 `advance_time_with_mode` 中：

```rust
// 自动放弃任何卡住的直播比赛
if state.state_manager.with_live_match(|_| ()).is_some() {
    if let Err(e) = abandon_match_logic(&state).await {
        println!("Warning: Failed to abandon stale live match: {}", e);
    }
}
```

**Live Match Session 存储** - 使用 `AppState`:

```rust
pub struct AppState {
    pub state_manager: StateManager,
    pub save_manager: std::sync::Mutex<SaveManager>,
    pub live_match: std::sync::Mutex<Option<LiveMatchSession>>,  // 新增
}
```

#### 同步时检查点

1. **新 API 是否需要 HTTP 接口？** - 如果上游添加了新命令
2. **LiveMatchSession 类型是否有变化？** - 检查 `create_live_match` 参数
3. **Game 结构体字段是否变化？** - 可能影响 `set_starting_xi` 等函数

---

### 2. `src-tauri/src/http_server/mod.rs` (145 行)

路由注册文件。格式：

```rust
pub fn create_routes() -> Router<AppState> {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/start_new_game", post(start_new_game))
        .route("/api/select_team", post(select_team))
        // ... 其他路由
}
```

**注意**：如果上游添加了新命令，需要在这里添加对应的 HTTP 路由。

---

### 3. `src-tauri/src/http_server/models.rs` (2 行)

参数结构体定义：

```rust
#[derive(Deserialize)]
pub struct AdvanceTimeModeParams {
    pub mode: Option<String>,
}
```

如果上游添加了新的命令参数，需要在这里添加对应的结构体。

---

### 4. `src-tauri/src/lib.rs` (11 行改动)

添加的内容：

```rust
#[cfg(feature = "web")]
pub mod http_server;

#[cfg(feature = "web")]
pub fn run_web(port: u16) {
    // 启动 HTTP 服务器
}
```

**几乎不会冲突**，因为这是新增代码。

---

### 5. `src-tauri/src/main.rs` (28 行改动)

添加 CLI 参数解析：

```rust
match (args.web, args.port) {
    (true, Some(port)) => {
        ofm_core::run_web(port);
    }
    // ...
}
```

**几乎不会冲突**，因为这是新增代码。

---

### 6. `vite.config.ts` (116 行改动)

关键配置：

```typescript
export default defineConfig({
  plugins: [react()],
  server: {
    port: 5173,
    host: true,  // 允许远程访问
    proxy: {
      '/api': {
        target: 'http://localhost:3001',
        changeOrigin: true,
      },
    },
  },
  build: {
    target: 'es2020',
  },
});
```

**同步时**：
- 保留 `server.proxy` 配置
- 合并其他上游配置

---

### 7. `src/lib/api.ts` (新文件)

前端 HTTP 适配器，模拟 Tauri `invoke()`：

```typescript
export async function invoke<T>(cmd: string, params?: Record<string, unknown>): Promise<T> {
  // 将 cmd 映射到 /api/cmd
  // 发送 POST 请求
  // 返回 JSON 响应
}
```

**必须保留**，这是前端与后端通信的核心。

---

### 8. `src/lib/tauri.ts` (新文件)

重新导出：

```typescript
export { invoke, isWeb } from './api';
```

---

### 9. `src/App.css` (145 行改动)

从 Tailwind v4 改回 v3 语法：

```css
/* v4 语法（上游可能使用）*/
@import "tailwindcss";

/* v3 语法（我们使用）*/
@tailwind base;
@tailwind components;
@tailwind utilities;
```

**可能冲突**：如果上游升级到 Tailwind v4，会再次冲突。

---

## 同步流程

### 步骤 1：获取上游更新

```bash
# 添加上游 remote（如果还没有）
git remote add upstream https://github.com/openfootmanager/openfootmanager.git

# 获取上游更新
git fetch upstream

# 切换到 web-migration 分支
git checkout web-migration
```

### 步骤 2：合并上游

```bash
# 合并上游 main
git merge upstream/main

# 或者使用 rebase（更干净但有风险）
git rebase upstream/main
```

### 步骤 3：解决冲突

#### 冲突类型 1：Cargo.toml
```toml
# 保留我们的依赖（axum, tokio 等）
# 合并上游新增的依赖
```

#### 冲突类型 2：package.json
```json
{
  "scripts": {
    "dev:web": "vite --mode web",  // 保留
    "dev": "vite",                  // 合并上游
    // ...
  }
}
```

#### 冲突类型 3：http_server/handlers.rs
```rust
// 如果 upstream 也有 handlers.rs 修改，可能需要手动合并
// 通常我们的改动在特定函数内，比较容易合并
```

### 步骤 4：重新编译测试

```bash
# 编译后端
cd src-tauri
cargo build --release --features web

# 启动后端
./target/release/openfootmanager --web --port 3001 &

# 编译前端
cd ..
npm run dev:web

# 测试验证
# 1. 访问 http://localhost:5173
# 2. 开始新游戏
# 3. 测试 tactics 页面修改阵容
# 4. 测试比赛模拟
```

---

## 验证清单

每次同步后，验证以下功能：

| # | 功能 | 测试步骤 | 预期结果 |
|---|------|----------|----------|
| 1 | 健康检查 | `curl http://localhost:3001/api/health` | 返回 `OK` |
| 2 | 开始新游戏 | 前端点击 "New Game" | 游戏初始化成功 |
| 3 | 选择球队 | 前端点击球队 | 进入 Dashboard |
| 4 | 推进时间 | 点击 "Continue" | 时间推进，比赛日触发直播 |
| 5 | 直播比赛 | 观看比赛模拟 | 比分、事件正常显示 |
| 6 | 完成比赛 | 点击 "Finish" | 报告生成，fixture 更新 |
| 7 | 修改阵容 | Tactics 页面修改后保存 | 刷新后阵容保持 |
| 8 | 修改阵型 | Tactics 页面修改阵型 | 刷新后阵型保持 |

---

## 潜在问题

### Q1: 上游添加了新命令怎么办？

**A**: 需要在 `http_server/handlers.rs` 中添加对应的处理函数，并在 `mod.rs` 中注册路由。

参考现有处理函数的写法：
```rust
pub async fn new_command(
    State(state): State<AppState>,
    Json(params): Json<Value>,
) -> Result<Json<Game>, String> {
    // 1. 解析参数
    // 2. 获取 game 状态
    // 3. 执行逻辑
    // 4. 保存并返回
    state.state_manager.set_game(game.clone());
    Ok(Json(game))
}
```

### Q2: 上游修改了 Game 结构体怎么办？

**A**: 
1. 如果是新增字段，自动兼容
2. 如果是删除字段，编译错误，需要移除相关代码
3. 如果是重命名字段，需要更新所有引用

### Q3: 上游升级到 Tailwind v4 怎么办？

**A**: 再次将配置改回 v3，或者：
1. 保持 v3 依赖（推荐，避免破坏）
2. 或者升级 Vite 到支持 rolldown 的版本（可能需要等待 LoongArch 支持）

### Q4: 上游重构了 LiveMatchSession 怎么办？

**A**: 检查 `src-tauri/src/application/live_match.rs` 和 `ofm_core::live_match_manager` 的 API 变化，更新 `handlers.rs` 中的调用。

---

## 备份与回滚

```bash
# 在同步前创建备份
git branch backup-before-sync-$(date +%Y%m%d)

# 如果同步失败，回滚
git reset --hard backup-before-sync-YYYYMMDD
```

---

## 联系方式

如有疑问，检查 `docs/WEB_VERSION.md` 了解使用方法。
