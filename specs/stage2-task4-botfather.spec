spec: task
name: "Moly BotFather Dialog Manager"
tags: [stage2, moly, botfather, ui]
---

## Intent

Implement a BotFather dialog manager at the Moly App layer -- a built-in special Bot
where users send /command commands to create, edit, and manage other Bots. BotFather
replicates Telegram BotFather's interaction model: conversational wizards, inline
button navigation, and instant feedback.

BotFather's dialog logic runs entirely locally (no network calls), invoking AITK's
Bot Store CRUD interfaces to operate on data.

## Constraints

- BotFather command parsing and state machine implemented at the Moly App layer, not in AITK
- BotFather is a built-in Bot that cannot be deleted or renamed; appears first in the
  chat list automatically on startup
- Command parsing supports `/`-prefixed commands and natural language input (prompts
  available commands when input doesn't match any command)
- All operations take effect immediately and are persisted to SQLite
- After creating a Bot, the new Bot automatically appears in the chat list

## Decided

- Dialog state machine uses an enum to represent the current state (Idle, AwaitingBotName,
  AwaitingUsername, etc.)
- /mybots uses an inline button list for display; clicking enters the Bot management submenu
- Username validation rules: 3-32 characters, lowercase letters/digits/underscores only,
  must end with `_bot` or `bot`
- BotFather replies include instructional text (how to paste the token into crew-rs)
- Bot avatars use a default generated icon (based on the first letter of the name);
  /setuserpic allows customization

## Boundaries

### Allowed to modify
- src/bot_manager/** (new directory)
- src/data/bots.rs (new file, Bot data types for App use)
- src/chat/ related files (add BotFather chat view)
- src/app_state.rs (add BotManager state)

### Forbidden
- Do not modify AITK library code (only call its interfaces)
- Do not modify existing Provider management flow (coexist in parallel)
- Do not add network requests (BotFather is purely local)

## Out of scope

- /setcommands (Bot command menu configuration -> Phase B)
- Bot avatar image processing/cropping
- BotFather internationalization (English only for now)

## Acceptance criteria

Scenario: /start shows welcome message and command list
  Test: test_botfather_start_command
  When the user sends "/start" to BotFather
  Then BotFather replies with a welcome message
  And the reply includes the available command list (/newbot, /mybots, /setname, etc.)

Scenario: /newbot complete creation flow
  Test: test_botfather_newbot_flow
  When the user sends "/newbot" to BotFather
  Then BotFather replies "Please give it a name"
  When the user inputs "Weather Assistant"
  Then BotFather replies "Please choose a username (must end with bot)"
  When the user inputs "weather_bot"
  Then BotFather replies with "Created" and the token
  And the new Bot appears in the chat list

Scenario: /newbot rejects invalid username
  Test: test_botfather_newbot_invalid_username
  Given BotFather is waiting for the user to input a username
  When the user inputs "invalid name with spaces"
  Then BotFather replies with username format requirements
  And the dialog state remains at awaiting username input

Scenario: /newbot rejects duplicate username
  Test: test_botfather_newbot_duplicate_username
  Given a Bot with username "weather_bot" already exists
  And BotFather is waiting for the user to input a username
  When the user inputs "weather_bot"
  Then BotFather replies that the username is already taken
  And the dialog state remains at awaiting username input

Scenario: /mybots lists Bots and supports button selection
  Test: test_botfather_mybots
  Given "2" Bots have been created: "Weather Assistant" and "Code Assistant"
  When the user sends "/mybots" to BotFather
  Then BotFather replies with "2" inline buttons
  When the user clicks the "Weather Assistant" button
  Then BotFather shows the management submenu (edit name, view Token, delete, etc.)

Scenario: /mybots shows prompt when no Bots exist
  Test: test_botfather_mybots_empty
  Given no Bots have been created
  When the user sends "/mybots" to BotFather
  Then BotFather replies "You haven't created any Bots yet" and suggests using /newbot

Scenario: /token shows token and usage instructions
  Test: test_botfather_token_command
  Given the "Weather Assistant" Bot has been selected for management
  When the user clicks the "View Token" button
  Then BotFather replies with that Bot's token
  And the reply includes crew-rs configuration instructions (API URL and token paste steps)

Scenario: /revoke regenerates token
  Test: test_botfather_revoke_token
  Given the "Weather Assistant" Bot has been selected for management with old token "1:moly_old"
  When the user clicks the "Reset Token" button
  Then BotFather confirms the token has been reset
  And displays the new token, which differs from the old token

Scenario: /setname modifies Bot name
  Test: test_botfather_setname
  Given the "Weather Assistant" Bot has been selected for management
  When the user clicks "Edit Name"
  Then BotFather replies "Please enter a new name"
  When the user inputs "Weather Forecast Master"
  Then BotFather confirms the name has been updated
  And the Bot name in the chat list is updated to "Weather Forecast Master"

Scenario: /deletebot requires confirmation to delete a Bot
  Test: test_botfather_deletebot_confirms
  Given the "Weather Assistant" Bot has been selected for management
  When the user clicks "Delete Bot"
  Then BotFather replies with a confirmation prompt ("Are you sure you want to delete?")
  When the user confirms deletion
  Then the Bot is removed from the chat list
  And associated message history is cleaned up

Scenario: Unrecognized command shows help
  Test: test_botfather_unknown_command
  When the user sends "just saying something" to BotFather
  Then BotFather replies suggesting to use /start to see available commands
