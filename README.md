# iced-borderless-window-resize-handle-widget

Invisible edge & corner resize handles for borderless [iced](https://iced.rs) windows.

Setting `decorations: false` in iced removes the OS window frame — including its native resize borders, which makes frameless windows impossible to resize with the mouse. This crate restores them with a single lightweight overlay widget.

Pressing a handle hands the drag to the operating system's own resize machinery via `drag_resize`, so resizing looks, feels, and behaves natively:

| Platform        | Mechanism                                                                                                                                                                                                                                            |
|-----------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Windows         | Native size loop (`WM_NCLBUTTONDOWN` + `HT*` hit-test codes). No `WM_NCHITTEST`/`WS_THICKFRAME` hacks, so window snapping is untouched. The resize cursor is also kept visible for the whole drag (a Windows modal-loop quirk — see the crate docs). |
| Linux (X11)     | `_NET_WM_MOVERESIZE` — the window manager drives the resize.                                                                                                                                                                                         |
| Linux (Wayland) | `xdg_toplevel.resize` — the compositor drives the resize.                                                                                                                                                                                            |
| macOS           | Inert. winit borderless windows already keep `NSWindowStyleMask::Resizable`, so macOS edge-resizes them natively.                                                                                                                                    |

The widget draws nothing and keeps no state — it only watches the borders of its layout bounds, shows the native resize cursors on hover, and captures clicks within its zones.

## Installation

```toml
[dependencies]
iced = "0.14"  # any feature set works — the widget is renderer-agnostic
iced_borderless_window_resize_handle_widget = "0.1"
```

The widget only needs iced's `advanced` feature (which many apps already enable) and forces no renderer or executor choices on you. Your app's binary keeps exactly the feature footprint you configured.

## Usage

Layer the handles on top of your UI in a `stack` and forward the message to this crate's `drag_resize` (a drop-in replacement for `iced::window::drag_resize` that also keeps the native resize cursor visible during the whole drag on Windows):

```rust,ignore
use iced::widget::stack;
use iced::window;
use iced_borderless_window_resize_handle_widget::{drag_resize, ResizeHandles};

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
            .map(|id| drag_resize(id, direction))
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

## Maximized windows

Resize handles make no sense while the window is maximized. Maximize/restore always resizes the window, so `window::resize_events` is the perfect trigger to re-query `window::is_maximized` and feed it to `.enabled(...)`:

```rust,ignore
// state: maximized: bool

// subscription:
window::resize_events().map(|(id, _)| Message::WindowResized(id)),

// update:
Message::WindowOpened(id) => id
    .map(|id| window::is_maximized(id).map(Message::Maximized))
    .unwrap_or_else(iced::Task::none),
Message::WindowResized(id) => window::is_maximized(id).map(Message::Maximized),
Message::Maximized(maximized) => {
    app.maximized = maximized;
    iced::Task::none()
}

// view:
ResizeHandles::new(Message::Resized).enabled(!app.maximized)
```

A disabled set of handles draws nothing, shows no cursor, and captures no clicks — content underneath behaves exactly as if the handles were not there.

## Notes

* The grab zones consume clicks in the outermost `gutter` pixels of the window — just like the invisible border of a native frame. Don't put interactive widgets flush against the window edges.
* Resizing a maximized window is ignored by the OS; gate the handles with `.enabled(...)` if you prefer.
* In multi-window apps, pass each window's `window::Id` into the message closure.

Run the demo with:

```text
cargo run --example basic
```
