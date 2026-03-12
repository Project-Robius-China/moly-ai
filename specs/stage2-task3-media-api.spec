spec: task
name: "AITK 媒体 API 端点"
tags: [stage2, aitk, telegram-api, media]
---

## 意图

为 AITK Telegram Bot API Server 添加媒体消息支持（图片、语音、音频、文档）。
crew-rs 的 TelegramChannel 使用 sendPhoto/sendVoice/sendAudio/sendDocument
发送媒体回复，使用 getFile 下载用户发送的媒体。本任务确保这些端点正确工作，
使 Moly 可以完整显示 crew-rs 的所有类型回复。

## 约束

- 同 Task 1: cfg gate + feature flag `telegram-server`
- 媒体文件存入 `{data_dir}/bot_media/{file_id}` 目录
- file_id 使用 UUID v4 生成
- 接收 multipart/form-data 格式（Telegram Bot API 标准）
- 单文件大小上限 20MB（与 Telegram 一致）
- 必须处理缺失文件的错误情况

## 已定决策

- multipart 解析: 使用 axum 内置的 `Multipart` extractor
- file_path 在 getFile 响应中为相对路径 `media/{file_id}`
- 文件下载端点: `GET /file/bot{token}/media/{file_id}`
- caption 与文件一起存储在 messages 表

## 边界

### 允许修改
- moly-aitk/src/telegram_server/api/send_photo.rs（新建）
- moly-aitk/src/telegram_server/api/send_voice.rs（新建）
- moly-aitk/src/telegram_server/api/send_audio.rs（新建）
- moly-aitk/src/telegram_server/api/send_document.rs（新建）
- moly-aitk/src/telegram_server/api/get_file.rs（新建）
- moly-aitk/src/telegram_server/api/mod.rs（添加路由）
- moly-aitk/src/telegram_server/models.rs（扩展媒体类型）

### 禁止
- 不修改 Task 1 已实现的 core 端点行为
- 不添加图片处理/转码依赖

## 排除范围

- 缩略图生成
- 视频消息
- 语音转文字

## 验收标准

场景: sendPhoto 存储图片并通知
  测试: test_send_photo_stores_file
  假设 已创建 Bot token "1:moly_abc"
  当 调用 `POST /bot1:moly_abc/sendPhoto` multipart 包含:
    | 字段     | 值               |
    | chat_id  | 1                |
    | photo    | test_image.jpg   |
    | caption  | 一张测试图片      |
  那么 响应状态码为 200
  并且 result.photo 数组非空
  并且 outbound channel 收到包含 photo 的消息

场景: sendVoice 存储语音
  测试: test_send_voice_stores_file
  假设 已创建 Bot token "1:moly_abc"
  当 调用 `POST /bot1:moly_abc/sendVoice` multipart 包含 voice 文件
  那么 响应状态码为 200
  并且 result.voice.file_id 为有效 UUID

场景: sendDocument 存储文档
  测试: test_send_document_stores_file
  假设 已创建 Bot token "1:moly_abc"
  当 调用 `POST /bot1:moly_abc/sendDocument` multipart 包含 document 文件
  那么 响应状态码为 200
  并且 result.document.file_name 与上传文件名一致

场景: sendAudio 存储音频
  测试: test_send_audio_stores_file
  假设 已创建 Bot token "1:moly_abc"
  当 调用 `POST /bot1:moly_abc/sendAudio` multipart 包含 audio 文件
  那么 响应状态码为 200
  并且 result.audio.file_id 为有效 UUID

场景: getFile 返回文件路径
  测试: test_get_file_returns_path
  假设 已通过 sendPhoto 上传文件，获得 file_id "uuid-1234"
  当 调用 `POST /bot1:moly_abc/getFile` body 为 `{"file_id": "uuid-1234"}`
  那么 响应状态码为 200
  并且 result.file_path 为 "media/uuid-1234"

场景: 文件下载端点
  测试: test_file_download
  假设 已通过 sendPhoto 上传文件，file_path 为 "media/uuid-1234"
  当 调用 `GET /file/bot1:moly_abc/media/uuid-1234`
  那么 响应状态码为 200
  并且 Content-Type 与文件 MIME 类型匹配
  并且 响应体为文件二进制内容

场景: 下载不存在的文件返回 404
  测试: test_file_download_not_found
  假设 已创建 Bot token "1:moly_abc"
  当 调用 `GET /file/bot1:moly_abc/media/nonexistent`
  那么 响应状态码为 404

场景: 超大文件被拒绝
  测试: test_reject_oversized_file
  假设 已创建 Bot token "1:moly_abc"
  当 上传一个大于 "20" MB 的文件
  那么 响应状态码为 413
