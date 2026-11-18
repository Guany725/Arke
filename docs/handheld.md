# Handheld mode (development implementation)

Baseline: Koimsurai/Arke `907192f5c8ca86e0667d28a54cdcebae5de231d1`.

In Settings > Appearance, choose Handheld interface mode. Controller input and
the hint bar can be switched independently. Desktop remains the default. Mode
and bindings use the existing settings persistence and import/export mechanism.

## Custom bindings

Expand Controller bindings in Appearance. Each of A, B, X, Y, LB, RB, LT, RT,
Menu, View, LS (left stick click), RS (right stick click) has an action dropdown.
Restore defaults clears overrides. Multiple controls may share an action.
Unassigned controls are allowed. The hint bar reflects the current bindings.

Actions include open, contextual back, parent folder, preview, menu, cut, copy,
rename, delete, paste, pin/unpin folder, add to shelf, remove from shelf,
copy/move shelf contents here, selection, new folder, refresh, address/search,
settings, tabs, page navigation, focus region and directional movement.

The directional pad and left stick always navigate. The right stick axes are
not used. Defaults are A open, B back, X menu, Y preview, LB/RB previous/next
tab, LT/RT page up/down, Menu menu, View next focus region; LS/RS unassigned.

Custom bindings apply to file browsing. Menus, confirmations, text input and
focus regions use the default controls, so remapping A or B cannot remove their
confirm/cancel path. Release held controls after reconnecting, activating the
window, changing bindings or entering/leaving a protected input context.
Only navigation repeats. Trigger hysteresis prevents threshold jitter from
repeating a file operation. Input goes only to the foreground Arke window.

Deleting uses Arke's normal delete behavior and existing confirmation settings.
Removing from the shelf removes a record, never its file. Copy/move shelf here
uses all existing shelf items eligible for the destination, not just the current
file selection. Pinning retains Arke's folder-only behavior.

Settings example (merge fields into existing settings; do not overwrite it):

```json
{
  "interface_mode": "handheld",
  "controller_enabled": true,
  "controller_hints": true,
  "controller_bindings": {
    "x": "copy",
    "y": "paste",
    "ls": "shelf_add",
    "rs": "shelf_remove"
  }
}
```

## Integration

`handheld_input.rs` decodes snapshots without depending on GPUI. The window owns
polling and region navigation in `workspace_handheld.rs`; explorer actions reuse
the existing operations in `explorer_handheld.rs`. No file engine is replaced.
Handheld rows use 52 logical pixels; common controls use at least 48. The shelf
keeps its original touch UI. View cycles Files > Sidebar > Toolbar > Files.
Toolbar shortcuts focus address/search and choose Details/Icons/Columns.
Long press opens the existing context menu. Text fields request the Windows
touch keyboard through InputPane; Windows controls whether it appears.

## Validation and remaining acceptance work

Standalone production decoder tests: 8 passed. Focus navigation tests: 1 passed.
The decoder harness omits only the Windows XInput polling function, so these
tests do not verify device discovery or GPUI integration. Rustfmt and diff
whitespace checks pass. Settings serialization tests were added but not run.

Full `cargo check -p arke-app --locked` has NOT passed. Offline dependency
resolution stops at missing `poly1305 0.9.1`; the available machine also lacks
the complete MSVC/Windows SDK toolchain. No executable or installer is provided.
Use the upstream documented Windows build environment, then run:

```text
cargo check -p arke-app --locked
cargo test -p arke-app --locked handheld
cargo clippy -p arke-app --locked -- -D warnings
```

Before release, check on a Windows 11 handheld:

- Build and launch; switch modes repeatedly; restart and import/export settings.
- Exercise every mapped operation on disposable files, including name conflicts,
  read-only files, recycle confirmation, shelf copy/move and pin/unpin.
- Hold buttons through menus, confirmation, focus loss and controller reconnect;
  verify that file actions never repeat and B never exits the application.
- Verify Details, Icons and Columns navigation and scrolling at 1280x720 and
  1920x1080 with 100%, 125% and 150% scaling and a large directory.
- Test touch long press, dragging, scrolling and the system keyboard on hardware.
- Recheck mouse/keyboard/Desktop mode and multiwindow behavior.

This is not a completed first-stage hardware acceptance. Settings expanders,
shelf rows and secondary dialogs still require touch/mouse for some interaction;
the dedicated controller regions cover the file list, destination list and
toolbar. Right-stick scrolling, exhaustive touch-target enlargement and tuning
of all narrow-window layouts remain outside this verified implementation.
