//! A minimal borderless window that can be moved *and* resized.
//!
//! Run with:
//!
//! ```text
//! cargo run --example basic
//! ```
use iced::alignment::Vertical;
use iced::mouse;
use iced::widget::{column, container, mouse_area, row, stack, text};
use iced::window;
use iced::window::settings::platform::CornerPreference;
use iced::{Color, Element, Length, Size, Subscription, Task};
use iced_borderless_window_resize_handle_widget::ResizeHandles;

const TITLEBAR_BACKGROUND: Color = Color::from_rgb8(0x1f, 0x25, 0x2b);
const BODY_BACKGROUND: Color = Color::from_rgb8(0xf2, 0xf0, 0xeb);
const DIM_TEXT: Color = Color::from_rgb8(0x5a, 0x62, 0x6b);
const LIGHT_TEXT: Color = Color::from_rgb8(0xe8, 0xea, 0xed);

fn main() -> iced::Result {
    iced::application(Basic::boot, update, view)
        .title(|_: &Basic| String::from("Resize handles — basic example"))
        .window(window::Settings {
            size: Size::new(720.0, 480.0),
            min_size: Some(Size::new(320.0, 240.0)),
            resizable: true,
            // Frameless: the OS frame — and its native resize borders — is
            // gone, which is exactly what `ResizeHandles` restores.
            decorations: false,
            #[cfg(windows)]
            platform_specific: window::settings::PlatformSpecific {
                corner_preference: CornerPreference::Default,
                undecorated_shadow: true,
                ..window::settings::PlatformSpecific::default()
            },
            ..window::Settings::default()
        })
        .subscription(subscription)
        .run()
}

#[derive(Debug, Clone)]
enum Message {
    WindowOpened(Option<window::Id>),
    WindowResized(Size),
    StartDrag,
    ToggleMaximize,
    Resize(window::Direction),
}

#[derive(Debug, Default)]
struct Basic {
    window_id: Option<window::Id>,
    window_size: Size,
}

impl Basic {
    fn boot() -> (Self, Task<Message>) {
        (
            Self::default(),
            // iced has no `window::Id::MAIN`; grab the id of the first window.
            window::latest().map(Message::WindowOpened),
        )
    }
}

fn subscription(_basic: &Basic) -> Subscription<Message> {
    Subscription::batch([
        window::open_events().map(|id| Message::WindowOpened(Some(id))),
        window::resize_events().map(|(_id, size)| Message::WindowResized(size)),
    ])
}

fn update(basic: &mut Basic, message: Message) -> Task<Message> {
    match message {
        Message::WindowOpened(id) => {
            if id.is_some() {
                basic.window_id = id;
            }
            Task::none()
        }
        Message::WindowResized(size) => {
            basic.window_size = size;
            Task::none()
        }
        Message::StartDrag => basic.window_id.map(window::drag).unwrap_or_else(Task::none),
        Message::ToggleMaximize => basic
            .window_id
            .map(window::toggle_maximize)
            .unwrap_or_else(Task::none),
        Message::Resize(direction) => basic
            .window_id
            .map(|id| window::drag_resize(id, direction))
            .unwrap_or_else(Task::none),
    }
}

fn view(basic: &Basic) -> Element<'_, Message> {
    let content = column![titlebar(), body(basic)]
        .width(Length::Fill)
        .height(Length::Fill);

    stack![content, ResizeHandles::new(Message::Resize)]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn titlebar() -> Element<'static, Message> {
    mouse_area(
        container(
            row![
                text("Borderless Window").size(15).color(LIGHT_TEXT),
                text("drag to move · double-click to maximize")
                    .size(12)
                    .color(DIM_TEXT),
            ]
            .spacing(12)
            .align_y(Vertical::Center),
        )
        .width(Length::Fill)
        .height(Length::Fixed(44.0))
        .padding([0, 14])
        .align_y(Vertical::Center)
        .style(|_| container::Style::default().background(TITLEBAR_BACKGROUND)),
    )
    .interaction(mouse::Interaction::Grab)
    .on_press(Message::StartDrag)
    .on_double_click(Message::ToggleMaximize)
    .into()
}

fn body(basic: &Basic) -> Element<'_, Message> {
    let live_size = if basic.window_size.width > 0.0 {
        format!(
            "{} × {}",
            basic.window_size.width.round(),
            basic.window_size.height.round()
        )
    } else {
        String::from("…")
    };

    container(
        column![
            text("Resize me from the edges").size(24),
            column![
                bullet("Move the cursor to any window edge or corner — it turns into a resize arrow."),
                bullet("Press and drag: the OS itself runs the native resize loop."),
                bullet("Corners have a larger grab area than edges."),
                bullet("The outermost few pixels of the window belong to the handles, like a native frame."),
            ]
            .spacing(8),
            row![text("Live size:").color(DIM_TEXT), text(live_size)]
                .spacing(8),
            text(platform_note()).size(12).color(DIM_TEXT),
        ]
        .spacing(18),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(24)
    .style(|_| container::Style::default().background(BODY_BACKGROUND).color(DIM_TEXT))
    .into()
}

fn bullet(line: &'static str) -> Element<'static, Message> {
    row![text("•"), text(line)].spacing(8).into()
}

fn platform_note() -> String {
    if cfg!(target_os = "macos") {
        "macOS: borderless windows already edge-resize natively; the overlay handles are inert here."
    } else if cfg!(target_os = "windows") {
        "Windows: resizing uses the native size loop — no WM_NCHITTEST tricks, so window snapping is untouched."
    } else {
        "Linux: resizing is driven by your WM (X11: _NET_WM_MOVERESIZE) or compositor (Wayland: xdg_toplevel.resize)."
    }
    .to_string()
}
