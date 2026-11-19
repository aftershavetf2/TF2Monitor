---
name: tf2monitor-ui
description: Build or review TF2Monitor's egui interface, including layout, visual hierarchy, labels, tooltips, theme, and UI performance.
---

# TF2Monitor UI

Start with the closest existing view or widget in `src/gui`. Reuse its
terminology and layout patterns. Check `src/gui/theme.rs` and
`src/gui/ui_utils.rs` for shared styles and helpers.

## Visual design

- Keep text, icons, and control states legible against their backgrounds in all
  supported themes. Pair meaningful colors with a label or icon, especially
  for player flags and status indicators.
- Use the shared `flag_checkbox` helpers in `src/gui/theme.rs` and
  `src/gui/menu_item.rs` for checkboxes tied to a player flag.
- Group related information and actions. Use placement, headings, and emphasis
  to make the most important information easy to find. Keep labels short and
  clear, and use tooltips for additional explanation.
- Use the shared typography roles in `src/gui/theme.rs`. Use `section_heading`
  for headings at the same level and `card_title` for titles inside cards.
- Use subtle tonal differences between named theme surfaces to group related
  content and controls. Use cards for self-contained items and compact rows
  where users need to compare values.
- Match button emphasis to the action's importance in the current view.
  Keep repeated secondary actions easy to find with a quieter style.
- Align related labels and controls along common edges. In the scoreboard,
  align comparable values consistently across rows.
- Follow existing padding and spacing patterns. Keep related items closer
  together and leave more space between groups. Use dividers where spacing
  alone leaves the boundary unclear.

## Responsiveness and CPU use

- Run blocking I/O and long-running work off the UI thread. Brief, infrequent
  operations triggered by an explicit user action may run synchronously if
  they complete without a noticeable pause. Keep blocking work out of per-frame
  rendering and periodic updates.
- Review what runs each time egui builds the UI. Reuse expensive results across
  frames and recompute them only when their inputs change. Keep CPU use low so
  TF2Monitor does not interfere with gameplay.

When you can run the app, open the affected view and check its layout,
readability, and responsiveness.
