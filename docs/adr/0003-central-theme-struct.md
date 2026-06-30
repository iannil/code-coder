# Central Theme Struct

Status: accepted

All colors used anywhere in the TUI are defined as fields on a single `Theme` struct held by `TuiApp`. No `Style::default().fg(Color::X)` calls are allowed in render code; everything reads from `app.theme.<role>`. The `Ctrl+T` toggle swaps the entire struct between `Theme::dark()` and `Theme::light()` in one assignment, replacing ~40 hardcoded color sites in `input_area`, `status_bar`, `dialogs`, and `message_list`.

## Why

Before this decision, `Ctrl+T` toggled only `markdown::set_dark_mode()` while 40+ other color sites in input_area, status_bar, dialogs, and message_list remained hardcoded. Pressing `Ctrl+T` produced a half-changed UI where assistant messages shifted color but borders, prompts, separators, popups, and status text stayed the same — visually worse than no theme switch at all.

Hardcoded colors also made it impossible to add new themes (high-contrast, solarized, etc.) without a sweep through every render file. A central struct turns theme additions into one new constructor + one new toggle branch.

## How to apply

- Color roles are named by **semantic purpose** (`user_msg`, `assistant_msg`, `dialog_warning`, `popup_selected_fg`, `separator`), not by literal color (`Yellow`, `Cyan`). Render code asks for the role; the theme provides the color.
- When introducing a new visual element, add a new role field to `Theme` rather than reusing an existing one — even if the colors happen to match today. Roles are cheap; conflation is expensive.
- `dark_mode: bool` on `TuiApp` is removed; the current mode is implicit in which `Theme` is loaded.
- The `markdown::set_dark_mode` global is replaced by passing the theme (or its markdown-relevant subset) into the markdown renderer.

## Considered Options

- **Delete `Ctrl+T` entirely** and ship dark-only. Rejected: theme switching is a real user need for accessibility (light-sensitive users) and external monitor scenarios. Removing the feature would defer, not solve, the problem.
- **Keep `Ctrl+T` as partial / experimental with a UI warning.** Rejected: a half-working feature is worse than none. Users who try it once and see inconsistent colors will assume the TUI is buggy.

## Consequences

- Adding a new theme = adding one constructor (`Theme::solarized_dark()`) and one option in a future `/theme` command. No render code changes.
- Adding a new color role = adding one field to `Theme` and updating both `dark()` and `light()` constructors. The compiler enforces completeness.
- Migration is a one-time sweep; afterward, hardcoded colors are a code-review blocker.
