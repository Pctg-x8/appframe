AppFrame
---

Cross-Platform GUI Application Framework on Rust  
For macOS/Windows

## Usage

### `GUIApplication::run(appname: &str, delegate: &mut impl EventDelegate) -> i32`

Runs the application. Returns exit code.

### `NativeWindowBuilder`

Build a `NativeWindow`.

- `new(width: u16, height: u16, caption: &'c str)`
  - create builder with client size and caption
- `closable(&mut self, bool) -> &mut Self`
  - Set window as closable(if true passed, default) or unclosable(if false passed)
- `resizable(&mut self, bool) -> &mut Self`
  - Set window as resizable(if true passed, default) or unresizable(if false passed)
- `create`
  - Create a window. Returns `None` if window is not presented by server.

### `NativeWindow::show`

Shows a window.

### `EventDelegate`

Delegated events from window server/system.

- `postinit(&mut self)`
  - called in `applicationDidFinishLaunching`.
