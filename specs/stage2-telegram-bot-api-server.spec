# Stage 2: Telegram Bot API Compatible Server
# Moly Bot-Native Messaging — BotFather + Telegram Bot API 兼容服务器

Status: design
Created: 2026-03-10
Target: 5/3 Demo

## Intent

将 Moly 从"开发者配置 Provider"模式转变为"Telegram 风格 Bot 管理"模式。
用户在 Moly 内通过 BotFather 对话创建 Bot、获得 token，将 token 粘贴到
crew-rs 的 Telegram 配置中，crew-rs 通过 teloxide 连接 Moly 的 Bot API
服务器，用户在 Moly 中直接与 Bot 聊天。

核心创新：Moly 实现 Telegram Bot API 兼容服务端，任何支持 Telegram Bot API
的框架（crew-rs、python-telegram-bot 等）都能接入。

## Architecture

### 层级划分

```
┌─────────────────────────────────────────────────────────┐
│                      Moly App (Makepad)                 │
│                                                         │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │
│  │ BotFather    │  │ Bot Chat     │  │ Bot Chat     │  │
│  │ Chat View    │  │ "天气助手"   │  │ "代码助手"   │  │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘  │
│         │                 │                 │           │
│  ┌──────▼─────────────────▼─────────────────▼────────┐  │
│  │              Moly Bot Manager (App 层)             │  │
│  │  • BotFather 对话逻辑（/newbot, /mybots, etc.）    │  │
│  │  • 用户消息 → push_update()                        │  │
│  │  • recv_outbound() → UI 显示 Bot 回复              │  │
│  └──────────────────────┬────────────────────────────┘  │
└─────────────────────────┼───────────────────────────────┘
                          │
┌─────────────────────────▼───────────────────────────────┐
│                    AITK (库层)                           │
│                                                         │
│  ┌─────────────────────────────────────────────────┐    │
│  │      Telegram Bot API Server (新模块)            │    │
│  │  http://localhost:{port}/bot{token}/{method}     │    │
│  │                                                  │    │
│  │  端点实现:                                       │    │
│  │  • getMe              — Bot 身份信息              │    │
│  │  • getUpdates         — 长轮询获取消息            │    │
│  │  • sendMessage        — 发送文本(+inline kbd)     │    │
│  │  • sendPhoto          — 发送图片                  │    │
│  │  • sendVoice          — 发送语音                  │    │
│  │  • sendAudio          — 发送音频                  │    │
│  │  • sendDocument       — 发送文档                  │    │
│  │  • editMessageText    — 编辑消息                  │    │
│  │  • deleteMessage      — 删除消息                  │    │
│  │  • answerCallbackQuery — 回调查询应答             │    │
│  │  • getFile / file download — 文件下载             │    │
│  │                                                  │    │
│  │  核心接口 (供 App 层调用):                        │    │
│  │  • push_update(bot_token, Update)                │    │
│  │  • recv_outbound(bot_token) -> OutboundMessage   │    │
│  │  • create_bot(name, ...) -> BotInfo + token      │    │
│  │  • list_bots() -> Vec<BotInfo>                   │    │
│  │  • update_bot(token, BotUpdate)                  │    │
│  │  • delete_bot(token)                             │    │
│  └────────────────────┬────────────────────────────┘    │
│                       │                                  │
│  ┌────────────────────▼────────────────────────────┐    │
│  │              Bot Store (SQLite)                  │    │
│  │  Tables:                                         │    │
│  │  • bots:     id, token, name, description,       │    │
│  │              username, photo, created_at          │    │
│  │  • messages: id, bot_id, chat_id, sender,        │    │
│  │              content, media_type, media_path,     │    │
│  │              reply_markup, timestamp              │    │
│  │  • media:    id, file_id, file_path, mime_type   │    │
│  └─────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
                          ▲
                          │ teloxide 长轮询
┌─────────────────────────┴───────────────────────────────┐
│                   crew-rs Gateway                        │
│  配置:                                                   │
│  • TELOXIDE_TELEGRAM_API_URL=http://localhost:{port}     │
│  • TELEGRAM_BOT_TOKEN=moly_{random_token}                │
│  • 其余配置不变（LLM provider、tools 等）               │
└─────────────────────────────────────────────────────────┘
```

### 消息流转

1. **用户在 Moly 中发送消息**
   → Moly Bot Manager 构造 Telegram `Update` 对象
   → 调用 AITK `push_update(bot_token, update)`
   → Update 进入 per-bot 队列

2. **crew-rs 拉取消息**
   → teloxide 调用 `POST /bot{token}/getUpdates` (长轮询)
   → AITK 服务器从队列中取出 Updates 返回
   → 如无消息，阻塞等待（timeout 后返回空数组）

3. **crew-rs 处理并回复**
   → Agent 推理 + 工具调用
   → teloxide 调用 `POST /bot{token}/sendMessage`
   → AITK 服务器接收，存入 messages 表
   → 通知 Moly `recv_outbound()` 有新消息

4. **Moly 显示回复**
   → Bot Manager 收到 OutboundMessage
   → 更新 Chat UI 显示 Bot 回复
   → 支持 inline keyboard 按钮渲染

## BotFather 对话设计

BotFather 是 Moly 中一个内置的特殊 Bot，出现在聊天列表中。

### 命令体系

```
基础命令:
/start     — 欢迎消息，展示可用命令
/newbot    — 创建新 Bot（对话式向导）
/mybots    — 列出所有 Bot（内联按钮选择）

编辑 Bot:
/setname        — 修改 Bot 名称
/setdescription — 修改 Bot 描述
/setabouttext   — 修改 Bot 简介
/setuserpic     — 修改 Bot 头像
/deletebot      — 删除 Bot

Bot 设置:
/token     — 查看 Bot token（用于粘贴到 crew-rs）
/revoke    — 重新生成 token
```

### /newbot 对话流程

```
用户: /newbot
BotFather: 好的，让我们创建一个新 Bot。请给它起个名字：

用户: 天气助手
BotFather: 很好。现在给它起个用户名（必须以 bot 结尾）：

用户: weather_bot
BotFather: ✅ 完成！你的新 Bot "天气助手" 已创建。

🔑 Token: moly_a1b2c3d4e5f6...

使用方法：
1. 复制上面的 Token
2. 在 crew-rs 网页的 Telegram 配置中粘贴
3. 设置 API URL 为: http://localhost:{port}
4. 启动 Gateway

你现在可以在聊天列表中找到 "天气助手" 开始对话。

使用 /mybots 管理你的 Bot，使用 /token 随时查看 Token。
```

### /mybots 交互

```
用户: /mybots
BotFather: 选择一个 Bot 进行管理：
[🤖 天气助手] [🤖 代码助手] [🤖 翻译Bot]

用户: (点击 天气助手)
BotFather: 🤖 天气助手 (@weather_bot)
选择操作：
[编辑名称] [编辑描述] [编辑头像]
[查看 Token] [重置 Token] [删除 Bot]
[◀ 返回]
```

## Telegram Bot API 实现细节

### Token 格式

Telegram 原始格式: `{bot_id}:{random_string}`
Moly 格式: `moly_{bot_id}_{random_hex}` (前缀区分来源)

### 长轮询实现 (getUpdates)

```
请求: POST /bot{token}/getUpdates
Body: { "offset": 12345, "timeout": 30, "limit": 100 }

逻辑:
1. 验证 token → 找到对应 Bot
2. 检查 update_queue 中 offset 之后的消息
3. 如果有消息 → 立即返回
4. 如果无消息 → 使用 tokio::select! 等待:
   a. 新消息到达 → 返回
   b. timeout 秒后 → 返回空数组 []
5. 返回 Telegram 标准 Response<Vec<Update>> 格式
```

### Telegram 数据类型 (需实现子集)

```rust
// 核心类型 (对应 Telegram Bot API)
struct User { id: i64, is_bot: bool, first_name: String, username: Option<String> }
struct Chat { id: i64, chat_type: String, title: Option<String> }
struct Message {
    message_id: i64,
    from: Option<User>,
    chat: Chat,
    date: i64,
    text: Option<String>,
    photo: Option<Vec<PhotoSize>>,
    voice: Option<Voice>,
    audio: Option<Audio>,
    document: Option<Document>,
    caption: Option<String>,
    reply_markup: Option<InlineKeyboardMarkup>,
}
struct Update { update_id: i64, message: Option<Message>, callback_query: Option<CallbackQuery> }
struct CallbackQuery { id: String, from: User, message: Option<Message>, data: Option<String> }
struct InlineKeyboardMarkup { inline_keyboard: Vec<Vec<InlineKeyboardButton>> }
struct InlineKeyboardButton { text: String, callback_data: Option<String>, url: Option<String> }

// 媒体类型
struct PhotoSize { file_id: String, file_unique_id: String, width: i32, height: i32 }
struct Voice { file_id: String, file_unique_id: String, duration: i32 }
struct Audio { file_id: String, file_unique_id: String, duration: i32, title: Option<String> }
struct Document { file_id: String, file_unique_id: String, file_name: Option<String> }
struct File { file_id: String, file_unique_id: String, file_path: Option<String> }
```

### API 端点详细

| 端点 | 方法 | 说明 | crew-rs 使用 |
|------|------|------|-------------|
| /bot{token}/getMe | GET/POST | 返回 Bot 信息 | 启动验证 |
| /bot{token}/getUpdates | POST | 长轮询获取消息 | 核心消息接收 |
| /bot{token}/sendMessage | POST | 发送文本+键盘 | 核心消息发送 |
| /bot{token}/sendPhoto | POST | 发送图片 | 媒体发送 |
| /bot{token}/sendVoice | POST | 发送语音 | 语音回复 |
| /bot{token}/sendAudio | POST | 发送音频 | 音频回复 |
| /bot{token}/sendDocument | POST | 发送文档 | 文件发送 |
| /bot{token}/editMessageText | POST | 编辑消息 | 消息更新 |
| /bot{token}/deleteMessage | POST | 删除消息 | 消息删除 |
| /bot{token}/answerCallbackQuery | POST | 回调应答 | 按钮交互 |
| /bot{token}/getFile | GET/POST | 获取文件信息 | 媒体下载 |
| /file/bot{token}/{file_path} | GET | 下载文件 | 媒体下载 |

## 数据存储 (SQLite)

### Schema

```sql
CREATE TABLE bots (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    token       TEXT NOT NULL UNIQUE,
    name        TEXT NOT NULL,
    username    TEXT NOT NULL UNIQUE,
    description TEXT DEFAULT '',
    about_text  TEXT DEFAULT '',
    photo_path  TEXT,
    created_at  DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at  DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE messages (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    bot_id       INTEGER NOT NULL REFERENCES bots(id),
    message_id   INTEGER NOT NULL,  -- per-bot auto-increment
    chat_id      INTEGER NOT NULL,
    sender_id    INTEGER NOT NULL,
    sender_name  TEXT,
    is_from_bot  BOOLEAN NOT NULL DEFAULT FALSE,
    content      TEXT,
    media_type   TEXT,  -- photo, voice, audio, document
    media_path   TEXT,
    caption      TEXT,
    reply_markup TEXT,  -- JSON serialized InlineKeyboardMarkup
    timestamp    DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(bot_id, message_id)
);

CREATE TABLE media (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    file_id        TEXT NOT NULL UNIQUE,
    file_unique_id TEXT NOT NULL,
    file_path      TEXT NOT NULL,
    mime_type      TEXT,
    file_size      INTEGER
);

CREATE INDEX idx_messages_bot_chat ON messages(bot_id, chat_id);
CREATE INDEX idx_messages_timestamp ON messages(timestamp);
```

## crew-rs 侧配置

crew-rs 只需在 Telegram channel 配置中支持自定义 API URL:

```json
{
  "gateway": {
    "channels": [
      {
        "type": "telegram",
        "token_env": "TELEGRAM_BOT_TOKEN",
        "base_url": "http://localhost:8088",
        "allowed_senders": ""
      }
    ]
  }
}
```

teloxide 支持通过 `Bot::from_env_with_client()` 或
`TELOXIDE_TELEGRAM_API_URL` 环境变量设置自定义 base URL。
crew-rs 需要的改动极小：在 `TelegramChannel::new()` 中传入 base_url 配置。

## Moly App 层设计

### BotFather 作为内置 Bot

- 启动时自动出现在聊天列表第一位
- 有特殊图标/标识（⚙️ 或 🤖 + 蓝色认证标记）
- 不可删除、不可重命名
- 对话逻辑完全在本地处理（不经过网络）

### Bot 聊天列表

创建的每个 Bot 自动出现在聊天列表中：
- 显示 Bot 名称 + 头像
- 显示连接状态（在线/离线，取决于 crew-rs 是否在轮询）
- 点击进入聊天界面
- 聊天界面与现有 Chat 界面复用

### 连接状态检测

- 当 crew-rs 调用 getUpdates 时，标记 Bot 为"在线"
- 如果超过 2 分钟无 getUpdates 请求，标记为"离线"
- UI 中显示绿点/灰点指示连接状态

## Boundary & Constraints

- AITK 的 Bot API 服务器不依赖 Makepad，纯 Rust + axum
- AITK 不处理 UI 逻辑，只提供消息队列接口
- Token 仅在本地有效，不可用于真实 Telegram
- 服务器仅监听 localhost，不暴露到网络
- SQLite 数据库文件存储在用户数据目录

## Out of Scope (本期不做)

- 真实 Telegram 代理（不转发到 api.telegram.org）
- Webhook 模式（只支持长轮询）
- Group chat 支持（只支持 1:1 对话）
- Payments / Stickers / Games API
- Bot 商店 / Bot 发现
- 多用户支持（单用户桌面应用）

## Decision Log

| # | Decision | Rationale |
|---|----------|-----------|
| 1 | BotFather 对话式而非设置页面 | 复刻 Telegram 体验，零 UI 学习成本 |
| 2 | 方案 A: 完整 Telegram Bot API 服务器 | 最大兼容性，crew-rs 零改动 |
| 3 | AITK 层实现 API 服务器 | 通用可复用，不绑定 Makepad |
| 4 | SQLite 持久化 | 支持消息历史和复杂查询 |
| 5 | 全功能复制 | 完整支持文本/媒体/键盘/编辑/删除 |
| 6 | 长轮询 (非 Webhook) | 桌面应用无公网 IP，与 crew-rs 现有模式一致 |

## Success Criteria

1. 用户通过 BotFather `/newbot` 在 30 秒内创建 Bot
2. Token 粘贴到 crew-rs 后，crew-rs 能成功连接并聊天
3. 支持文本消息双向通信
4. 支持 crew-rs 发送的 inline keyboard 按钮交互
5. 支持媒体消息（图片、语音、文档）
6. Bot 连接状态实时显示
7. 消息历史持久化存储，重启后可恢复
