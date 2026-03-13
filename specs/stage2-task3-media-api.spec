spec: task
name: "AITK Media API Endpoints"
tags: [stage2, aitk, telegram-api, media]
---

## Intent

Add media message support (images, voice, audio, documents) to the AITK Telegram
Bot API Server. Octos's TelegramChannel uses sendPhoto/sendVoice/sendAudio/sendDocument
to send media replies and getFile to download user-sent media. This task ensures these
endpoints work correctly so that Moly can fully display all types of Octos replies.

## Constraints

- Same as Task 1: cfg gate + feature flag `telegram-server`
- Media files stored in `{data_dir}/bot_media/{file_id}` directory
- file_id generated using UUID v4
- Accepts multipart/form-data format (Telegram Bot API standard)
- Single file size limit of 20MB (consistent with Telegram)
- Must handle missing file error cases

## Decided

- Multipart parsing: use axum's built-in `Multipart` extractor
- file_path in getFile response is a relative path `media/{file_id}`
- File download endpoint: `GET /file/bot{token}/media/{file_id}`
- caption is stored alongside the file in the messages table

## Boundaries

### Allowed to modify
- moly-aitk/src/telegram_server/api/send_photo.rs (new file)
- moly-aitk/src/telegram_server/api/send_voice.rs (new file)
- moly-aitk/src/telegram_server/api/send_audio.rs (new file)
- moly-aitk/src/telegram_server/api/send_document.rs (new file)
- moly-aitk/src/telegram_server/api/get_file.rs (new file)
- moly-aitk/src/telegram_server/api/mod.rs (add routes)
- moly-aitk/src/telegram_server/models.rs (extend media types)

### Forbidden
- Do not modify core endpoint behavior already implemented in Task 1
- Do not add image processing/transcoding dependencies

## Out of scope

- Thumbnail generation
- Video messages
- Voice-to-text

## Acceptance criteria

Scenario: sendPhoto stores image and notifies
  Test: test_send_photo_stores_file
  Given a Bot with token "1:moly_abc" has been created
  When `POST /bot1:moly_abc/sendPhoto` is called with multipart containing:
    | Field    | Value            |
    | chat_id  | 1                |
    | photo    | test_image.jpg   |
    | caption  | A test image     |
  Then the response status code is 200
  And result.photo array is non-empty
  And outbound channel receives a message containing photo

Scenario: sendVoice stores voice
  Test: test_send_voice_stores_file
  Given a Bot with token "1:moly_abc" has been created
  When `POST /bot1:moly_abc/sendVoice` is called with multipart containing a voice file
  Then the response status code is 200
  And result.voice.file_id is a valid UUID

Scenario: sendDocument stores document
  Test: test_send_document_stores_file
  Given a Bot with token "1:moly_abc" has been created
  When `POST /bot1:moly_abc/sendDocument` is called with multipart containing a document file
  Then the response status code is 200
  And result.document.file_name matches the uploaded file name

Scenario: sendAudio stores audio
  Test: test_send_audio_stores_file
  Given a Bot with token "1:moly_abc" has been created
  When `POST /bot1:moly_abc/sendAudio` is called with multipart containing an audio file
  Then the response status code is 200
  And result.audio.file_id is a valid UUID

Scenario: getFile returns file path
  Test: test_get_file_returns_path
  Given a file has been uploaded via sendPhoto with file_id "uuid-1234"
  When `POST /bot1:moly_abc/getFile` is called with body `{"file_id": "uuid-1234"}`
  Then the response status code is 200
  And result.file_path is "media/uuid-1234"

Scenario: File download endpoint
  Test: test_file_download
  Given a file has been uploaded via sendPhoto with file_path "media/uuid-1234"
  When `GET /file/bot1:moly_abc/media/uuid-1234` is called
  Then the response status code is 200
  And Content-Type matches the file's MIME type
  And the response body is the binary file content

Scenario: Downloading a non-existent file returns 404
  Test: test_file_download_not_found
  Given a Bot with token "1:moly_abc" has been created
  When `GET /file/bot1:moly_abc/media/nonexistent` is called
  Then the response status code is 404

Scenario: Oversized file is rejected
  Test: test_reject_oversized_file
  Given a Bot with token "1:moly_abc" has been created
  When a file larger than "20" MB is uploaded
  Then the response status code is 413
