# Known Issues & Future UX Improvements

## Bug: Code Block Copy Button Not Working

**Status:** Open
**Scope:** moly-kit (affects all chats, not BotFather-specific)

The copy button on markdown code blocks does not copy to clipboard when clicked.
The button is visible but produces no effect.

**Handler location:** `moly-kit/src/widgets/messages.rs` (~line 762)

The `widget_action(ids!(copy_code_button))` event may not be reaching the handler,
or the `code_view` widget lookup may be failing. Needs investigation.

---

## Decision: QuickReply Buttons Deferred for BotFather

**Status:** Deferred
**Context:** BotFather interactive responses (stage2-task4c)

### What was tried

- Quick reply buttons were implemented for `/mybots` (clickable bot list)
  and `/start` (command shortcuts).
- Three bugs were discovered and fixed in the QuickReplyGroup widget:
  1. PortalList with `height: Fit` doesn't render — replaced with ComponentMap
  2. `cx.action()` not reaching Messages — switched to `cx.widget_action()`
  3. `disabled` state not resetting on widget reuse — added reset in `set_buttons`

### Decision

Removed all quick reply buttons from BotFather in favor of text + number
interaction. Reasons:

- Simpler UX, no visual clutter
- Avoids scalability issues (many bots would overflow single-row layout)
- Quick reply widget still needs work for production quality

### Future improvements (when revisiting quick replies)

- **Pagination:** `/mybots` with many bots should paginate (5 per page
  with Next/Prev navigation)
- **Multi-row layout:** QuickReplyGroup should support wrapping to
  multiple rows instead of fixed height
- **Inline keyboards:** Telegram-style grid buttons (2-column layout)
  would be a better fit for bot management menus
- **Copy button:** Dedicated copy-to-clipboard widget for tokens
  (not relying on markdown code block copy)
