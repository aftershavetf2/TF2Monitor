# AGENTS.md

## Language

- Use US English in project files and user-facing text.
- Keep wording concise and easy to understand.

## Rust

- Handle `Option` and `Result` explicitly when a value can be missing or an
  operation can fail. Prefer `match` or `if let` when handling cases locally.
- Use `unwrap()` only when the code makes it clear why it cannot fail.
- Prefer early returns to reduce nesting. Use `let ... else` when a pattern
  must match to continue, and `?` to propagate errors or missing values.
- Give each function one clear responsibility. When a function mixes distinct
  steps, such as reading data, applying rules, and rendering the result,
  extract those steps into focused helpers so the caller reads in workflow order.
- Name helpers after the action they perform and the data they act on, such
  as `load_legacy_markings` or `render_player_rows`.
- Review functions with deep nesting or many branches. Extract a block when
  the helper's name and inputs make the caller easier to follow.
- Use the **Rust: Top 10 by Cyclomatic Complexity** VS Code task to find
  functions worth reviewing. Use complexity scores as a review aid and judge
  each proposed split by how it affects readability.

## User data

- Preserve user-created player markings across upgrades, migrations, and
  storage changes.
- Provide menu actions to export and import player markings as JSON.
  Import must preserve existing markings unless the user explicitly chooses
  to replace them.
- Data that can be fetched again from Steam, SourceBans, or TF2BD may be
  cached in a database when useful.

## User interface

- Use the [tf2monitor-ui skill](.agents/skills/tf2monitor-ui/SKILL.md) for
  egui changes and reviews.

## Verification

- Run `cargo clippy` and relevant tests locally on Linux for Rust changes.
- If a change affects Windows-specific behavior, state in the summary
  whether that behavior was verified on Windows.
