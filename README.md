# iced-borderless-window-resize-handle-widget

Invisible edge & corner resize handles for borderless [iced](https://iced.rs) windows.

Setting `decorations: false` in iced removes the OS window frame — including its native resize borders, which makes frameless windows impossible to resize with the mouse. This crate restores them with a single lightweight overlay widget.

Pressing a handle hands the drag to the operating system's own resize machinery via `iced::window::drag_resize`, so resizing looks, feels, and behaves natively:

| Platform | Mechanism |
| --- | --- |
| Windows | Native size loop (`WM_NCLBUTTONDOWN` + `HT*` hit-test codes). No `WM_NCHITTEST`/`WS_THICKFRAME` hacks, so window snapping is untouched. |
| Linux (X11) | `_NET_WM_MOVERESIZE` — the window manager drives the resize. |
| Linux (Wayland) | `xdg_toplevel.resize` — the compositor drives the resize. |
| macOS | Inert. winit borderless windows already keep `NSWindowStyleMask::Resizable`, so macOS edge-resizes them natively. |

The widget draws nothing and keeps no state — it only watches the borders of its layout bounds, shows the native resize cursors on hover, and captures clicks within its zones.

## Installation

```toml
[dependencies]
iced = "0.14"
iced_borderless_window_resize_handle_widget = "0.1"
```

## Usage

Layer the handles on top of your UI in a `stack` and forward the message to `iced::window::drag_resize`:

```rust,ignore
use iced::widget::stack;
use iced::window;
use iced_borderless_window_resize_handle_widget::ResizeHandles;

#[derive(Debug, Clone)]
enum Message {
    WindowOpened(Option<window::Id>),
    Resized(window::Direction),
}

fn view(state: &App) -> iced::Element<'_, Message> {
    stack![
        my_ui(state),
        ResizeHandles::new(Message::Resized),
    ]
    .width(iced::Fill)
    .height(iced::Fill)
    .into()
}

fn update(app: &mut App, message: Message) -> iced::Task<Message> {
    match message {
        Message::WindowOpened(id) => {
            app.window_id = id;
            iced::Task::none()
        }
        Message::Resized(direction) => app
            .window_id
            .map(|id| window::drag_resize(id, direction))
            .unwrap_or_else(iced::Task::none),
    }
}
```

iced has no `window::Id::MAIN`; capture the id of your main window once at boot:

```rust,ignore
fn boot() -> (App, iced::Task<Message>) {
    (App::default(), window::latest().map(Message::WindowOpened))
}
```

If your message enum implements `From<window::Direction>`, you can skip the closure:

```rust,ignore
impl From<window::Direction> for Message {
    fn from(direction: window::Direction) -> Self {
        Message::Resized(direction)
    }
}

// later, in `view`:
stack![my_ui(state), ResizeHandles::from_direction()]
```

Or wrap any element in one call:

```rust,ignore
with_resize_handles(my_ui(state), Message::Resized)
```

## Options

```rust,ignore
ResizeHandles::new(Message::Resized)
    .gutter(6.0)   // edge zone thickness (logical px, default 6)
    .corner(14.0)  // corner zone size — corners take precedence (default 14)
    .enabled(true) // e.g. `.enabled(!is_maximized)` to disable while maximized
```

## Notes

* The grab zones consume clicks in the outermost `gutter` pixels of the window — just like the invisible border of a native frame. Don't put interactive widgets flush against the window edges.
* Resizing a maximized window is ignored by the OS; gate the handles with `.enabled(...)` if you prefer.
* In multi-window apps, pass each window's `window::Id` into the message closure.

Run the demo with:

```text
cargo run --example basic
```
