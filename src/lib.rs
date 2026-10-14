//! Invisible resize handles for borderless [iced] windows.
//!
//! Setting `decorations: false` on an iced window removes the operating
//! system's window frame — and with it the native resize borders, making the
//! window impossible to resize with the mouse. This crate restores them with
//! a single overlay widget: [`ResizeHandles`] places eight invisible grab
//! zones (four edges and four larger corners) along the borders of a window.
//!
//! Hovering a zone shows the matching native resize cursor, and pressing one
//! publishes a message that the app maps to [`iced::window::drag_resize`],
//! which hands the drag to the operating system's own resize machinery:
//!
//! | Platform  | Resize mechanism                                                    |
//! | --------- | ------------------------------------------------------------------- |
//! | Windows   | Native size loop (`WM_NCLBUTTONDOWN` + `HT*` hit-test codes).        |
//! | Linux/X11 | `_NET_WM_MOVERESIZE` — the window manager runs the resize.           |
//! | Linux/Wayland | `xdg_toplevel.resize` — the compositor runs the resize.          |
//! | macOS     | Inert; borderless windows already edge-resize natively (winit keeps `NSWindowStyleMask::Resizable`). |
//!
//! No platform-specific window hacks are involved: the widget never touches
//! `WM_NCHITTEST`, `WS_THICKFRAME`, or anything else that could interfere
//! with native window snapping.
//!
//! [iced]: https://iced.rs
//!
//! # Usage
//!
//! Layer the handles on top of your UI in a [`stack`](iced::widget::stack) and
//! map the published message to [`iced::window::drag_resize`]:
//!
//! ```ignore
//! use iced::application;
//! use iced::widget::{stack, text};
//! use iced::window;
//! use iced_borderless_window_resize_handle_widget::{drag_resize, ResizeHandles};
//!
//! fn main() -> iced::Result {
//!     application(Example::boot, update, view)
//!         .window(window::Settings {
//!             decorations: false, // frameless: no native resize borders
//!             ..window::Settings::default()
//!         })
//!         .run()
//! }
//!
//! #[derive(Debug, Clone)]
//! enum Message {
//!     WindowOpened(Option<window::Id>),
//!     Resized(window::Direction),
//! }
//!
//! #[derive(Default)]
//! struct Example {
//!     window_id: Option<window::Id>,
//! }
//!
//! impl Example {
//!     fn boot() -> (Self, iced::Task<Message>) {
//!         // iced has no `window::Id::MAIN`; grab the id of the first window.
//!         (Self::default(), window::latest().map(Message::WindowOpened))
//!     }
//! }
//!
//! fn update(example: &mut Example, message: Message) -> iced::Task<Message> {
//!     match message {
//!         Message::WindowOpened(id) => {
//!             example.window_id = id;
//!             iced::Task::none()
//!         }
//!         Message::Resized(direction) => example
//!             .window_id
//!             // `drag_resize` is a drop-in replacement for
//!             // `iced::window::drag_resize` that also keeps the native resize
//!             // cursor visible during the whole drag on Windows.
//!             .map(|id| drag_resize(id, direction))
//!             .unwrap_or_else(iced::Task::none),
//!     }
//! }
//!
//! fn view(_example: &Example) -> iced::Element<'_, Message> {
//!     stack![
//!         text("Hello! Try resizing me from the edges."),
//!         ResizeHandles::new(Message::Resized),
//!     ]
//!     .width(iced::Fill)
//!     .height(iced::Fill)
//!     .into()
//! }
//! ```
//!
//! If your message enum implements `From<window::Direction>`, you can skip the
//! closure entirely with [`ResizeHandles::from_direction`].
//!
//! For a one-liner, [`with_resize_handles`] wraps any element in the
//! `stack` for you:
//!
//! ```ignore
//! use iced_borderless_window_resize_handle_widget::with_resize_handles;
//!
//! fn view(state: &State) -> iced::Element<'_, Message> {
//!     with_resize_handles(my_ui(state), Message::Resized)
//! }
//! ```
//!
//! # Notes
//!
//! * The grab zones consume clicks in the outermost [`ResizeHandles::gutter`]
//!   pixels of the window — just like the invisible border of a native window
//!   frame. Avoid placing interactive widgets flush against the window edges.
//! * Resizing a maximized window is ignored by the OS. If you prefer to hide
//!   the handles while maximized, subscribe to the window state and use
//!   [`ResizeHandles::enabled`] (see [`iced::window::is_maximized`]).
//! * In multi-window applications, capture the [`iced::window::Id`] of each
//!   window (e.g. with [`iced::window::latest`] or
//!   [`iced::window::open_events`]) and pass it into the message closure.

#![warn(missing_docs)]

mod platform;
mod resize_handles;

pub use platform::drag_resize;
pub use resize_handles::ResizeHandles;

use iced::advanced::renderer;
use iced::widget::Stack;
use iced::window;
use iced::{Element, Length};

/// Wraps the given content in a [`Stack`] with [`ResizeHandles`] layered on
/// top, filling the available space.
///
/// This is a convenience shorthand for:
///
/// ```ignore
/// stack![
///     content,
///     ResizeHandles::new(on_resize),
/// ]
/// .width(Length::Fill)
/// .height(Length::Fill)
/// ```
///
/// # Example
///
/// ```ignore
/// use iced_borderless_window_resize_handle_widget::with_resize_handles;
///
/// fn view(state: &State) -> iced::Element<'_, Message> {
///     with_resize_handles(my_ui(state), Message::Resized)
/// }
/// ```
pub fn with_resize_handles<'a, Message, Theme, Renderer>(
    content: impl Into<Element<'a, Message, Theme, Renderer>>,
    on_resize: impl Fn(window::Direction) -> Message + 'a,
) -> Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: 'a + renderer::Renderer,
{
    Stack::with_children(vec![
        content.into(),
        ResizeHandles::new(on_resize).into(),
    ])
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
