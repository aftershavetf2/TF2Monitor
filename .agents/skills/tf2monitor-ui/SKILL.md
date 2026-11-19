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
- Group related information and actions. Use placement, headings, and emphasis
  to make the most important information easy to find. Keep labels short and
  clear, and use tooltips for additional explanation.
- Align related labels and controls along common edges. In the scoreboard,
  align comparable values consistently across rows.
- Follow existing padding and spacing patterns. Keep related items closer
  together and leave more space between groups. Use dividers where spacing
  alone leaves the boundary unclear.

## Responsiveness and CPU use

- Run blocking I/O and other long-running work off the UI thread.
- Review what runs each time egui builds the UI. Reuse expensive results across
  frames and recompute them only when their inputs change. Keep CPU use low so
  TF2Monitor does not interfere with gameplay.

When you can run the app, open the affected view and check its layout,
readability, and responsiveness.
