//! The [`ResizeHandles`] overlay widget.

use iced::advanced::layout::{Layout, Limits, Node};
use iced::advanced::mouse::{self, Cursor, Interaction};
use iced::advanced::renderer;
use iced::advanced::widget::{Tree, Widget};
use iced::advanced::{Clipboard, Shell};
use iced::touch;
use iced::window;
use iced::{Element, Event, Length, Point, Rectangle, Size};

/// An invisible overlay that adds resize handles along the edges and corners
/// of a window.
///
/// [`ResizeHandles`] is meant to be layered on top of your UI, typically as the
/// last child of a [`stack`](iced::widget::stack):
///
/// ```ignore
/// stack![
///     my_content,
///     ResizeHandles::new(|direction| Message::Resize(direction)),
/// ]
/// ```
///
/// Hovering an edge or corner shows the matching native resize cursor, and
/// pressing one publishes a message that your app maps to
/// [`iced::window::drag_resize`], handing the drag to the operating system's
/// own resize loop.
///
/// The widget never draws anything and keeps no state; it only reacts to mouse
/// and touch input near the borders of its layout bounds. It does not use any
/// platform-specific window hacks (such as `WM_NCHITTEST` overrides or
/// `WS_THICKFRAME` on Windows), so native window snapping keeps working.
pub struct ResizeHandles<'a, Message> {
    on_resize: Box<dyn Fn(window::Direction) -> Message + 'a>,
    gutter: f32,
    corner: f32,
    enabled: bool,
}

impl<'a, Message> ResizeHandles<'a, Message> {
    /// The default thickness of the edge zones, in logical pixels.
    pub const DEFAULT_GUTTER: f32 = 6.0;

    /// The default size of the corner zones, in logical pixels.
    pub const DEFAULT_CORNER: f32 = 14.0;

    /// Creates new resize handles that publish the message returned by
    /// `on_resize` when a handle is pressed.
    ///
    /// The `window::Direction` describes which edge or corner was grabbed and
    /// can be passed straight to [`iced::window::drag_resize`].
    pub fn new(on_resize: impl Fn(window::Direction) -> Message + 'a) -> Self {
        Self {
            on_resize: Box::new(on_resize),
            gutter: Self::DEFAULT_GUTTER,
            corner: Self::DEFAULT_CORNER,
            enabled: true,
        }
    }

    /// Sets the thickness of the edge zones, in logical pixels.
    ///
    /// Defaults to [`Self::DEFAULT_GUTTER`].
    pub fn gutter(mut self, gutter: f32) -> Self {
        self.gutter = gutter;
        self
    }

    /// Sets the size of the corner zones, in logical pixels.
    ///
    /// Corners are squares and take precedence over the edges, so they should
    /// generally be at least as large as the gutter. Defaults to
    /// [`Self::DEFAULT_CORNER`].
    pub fn corner(mut self, corner: f32) -> Self {
        self.corner = corner;
        self
    }

    /// Sets whether the handles are active.
    ///
    /// When disabled, the widget neither shows resize cursors nor captures
    /// clicks. Useful to turn the handles off while the window is maximized:
    ///
    /// ```ignore
    /// ResizeHandles::new(Message::Resize)
    ///     .enabled(!state.is_maximized)
    /// ```
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

impl<'a, Message> ResizeHandles<'a, Message>
where
    Message: From<window::Direction> + 'a,
{
    /// Creates resize handles for any message type that can be built from a
    /// [`window::Direction`].
    ///
    /// A convenient shorthand for [`Self::new`] when your message enum
    /// implements `From<window::Direction>`:
    ///
    /// ```ignore
    /// impl From<window::Direction> for Message {
    ///     fn from(direction: window::Direction) -> Self {
    ///         Message::Resize(direction)
    ///     }
    /// }
    ///
    /// // later, in `view`:
    /// stack![my_content, ResizeHandles::from_direction()]
    /// ```
    pub fn from_direction() -> Self {
        Self::new(Message::from)
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for ResizeHandles<'_, Message>
where
    Renderer: renderer::Renderer,
{
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &Limits,
    ) -> Node {
        let limits = limits.width(Length::Fill).height(Length::Fill);

        Node::new(limits.resolve(Length::Fill, Length::Fill, Size::ZERO))
    }

    fn update(
        &mut self,
        _tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        if !self.enabled || shell.is_event_captured() {
            return;
        }

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerPressed { .. }) => {
                let bounds = layout.bounds();

                if let Some(direction) = cursor.position().and_then(|position| {
                    zone_at(position, bounds, self.gutter, self.corner)
                }) {
                    shell.publish((self.on_resize)(direction));
                    // Native-frame behavior: a press on a resize border never
                    // reaches the content underneath.
                    shell.capture_event();
                }
            }
            _ => {}
        }
    }

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        layout: Layout<'_>,
        cursor: Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> Interaction {
        if !self.enabled {
            return Interaction::None;
        }

        let bounds = layout.bounds();

        cursor
            .position()
            .and_then(|position| {
                zone_at(position, bounds, self.gutter, self.corner)
            })
            .map(interaction_for)
            .unwrap_or(Interaction::None)
    }

    fn draw(
        &self,
        _tree: &Tree,
        _renderer: &mut Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        _layout: Layout<'_>,
        _cursor: Cursor,
        _viewport: &Rectangle,
    ) {
        // Invisible: the handles are hit areas only and never draw anything.
    }
}

impl<'a, Message, Theme, Renderer>
    From<ResizeHandles<'a, Message>> for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: 'a + renderer::Renderer,
{
    fn from(handles: ResizeHandles<'a, Message>) -> Self {
        Element::new(handles)
    }
}

/// Returns the resize zone containing `position` within `bounds`, if any.
///
/// Positions outside `bounds` never match. Corner zones are `corner`-sized
/// squares and take precedence over the `gutter`-thick edge zones.
fn zone_at(
    position: Point,
    bounds: Rectangle,
    gutter: f32,
    corner: f32,
) -> Option<window::Direction> {
    let left = position.x - bounds.x;
    let top = position.y - bounds.y;
    let right = (bounds.x + bounds.width) - position.x;
    let bottom = (bounds.y + bounds.height) - position.y;

    // The cursor must be over the widget for any zone to match.
    if left < 0.0 || top < 0.0 || right < 0.0 || bottom < 0.0 {
        return None;
    }

    let is_near = |distance: f32, limit: f32| distance <= limit;

    // Corners first: they get the larger grab area.
    if is_near(left, corner) && is_near(top, corner) {
        return Some(window::Direction::NorthWest);
    }
    if is_near(right, corner) && is_near(top, corner) {
        return Some(window::Direction::NorthEast);
    }
    if is_near(left, corner) && is_near(bottom, corner) {
        return Some(window::Direction::SouthWest);
    }
    if is_near(right, corner) && is_near(bottom, corner) {
        return Some(window::Direction::SouthEast);
    }

    if is_near(left, gutter) {
        Some(window::Direction::West)
    } else if is_near(right, gutter) {
        Some(window::Direction::East)
    } else if is_near(top, gutter) {
        Some(window::Direction::North)
    } else if is_near(bottom, gutter) {
        Some(window::Direction::South)
    } else {
        None
    }
}

/// Maps a resize direction to the matching native resize cursor.
fn interaction_for(direction: window::Direction) -> Interaction {
    match direction {
        window::Direction::North | window::Direction::South => {
            Interaction::ResizingVertically
        }
        window::Direction::East | window::Direction::West => {
            Interaction::ResizingHorizontally
        }
        window::Direction::NorthEast | window::Direction::SouthWest => {
            Interaction::ResizingDiagonallyUp
        }
        window::Direction::NorthWest | window::Direction::SouthEast => {
            Interaction::ResizingDiagonallyDown
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GUTTER: f32 = ResizeHandles::<()>::DEFAULT_GUTTER;
    const CORNER: f32 = ResizeHandles::<()>::DEFAULT_CORNER;

    const BOUNDS: Rectangle =
        Rectangle::new(Point::ORIGIN, Size::new(400.0, 300.0));

    fn zone_at_default(position: Point) -> Option<window::Direction> {
        zone_at(position, BOUNDS, GUTTER, CORNER)
    }

    #[test]
    fn interior_returns_none() {
        assert!(zone_at_default(Point::new(200.0, 150.0)).is_none());
        assert!(zone_at_default(Point::new(10.0, 150.0)).is_none());
        assert!(zone_at_default(Point::new(200.0, 300.0 - CORNER)).is_none());
    }

    #[test]
    fn outside_bounds_returns_none() {
        assert!(zone_at_default(Point::new(-3.0, 150.0)).is_none());
        assert!(zone_at_default(Point::new(403.0, 150.0)).is_none());
        assert!(zone_at_default(Point::new(200.0, -3.0)).is_none());
        assert!(zone_at_default(Point::new(200.0, 303.0)).is_none());
    }

    #[test]
    fn edges_match_within_the_gutter() {
        assert!(matches!(
            zone_at_default(Point::new(3.0, 150.0)),
            Some(window::Direction::West)
        ));
        assert!(matches!(
            zone_at_default(Point::new(400.0 - 3.0, 150.0)),
            Some(window::Direction::East)
        ));
        assert!(matches!(
            zone_at_default(Point::new(200.0, 3.0)),
            Some(window::Direction::North)
        ));
        assert!(matches!(
            zone_at_default(Point::new(200.0, 300.0 - 3.0)),
            Some(window::Direction::South)
        ));
    }

    #[test]
    fn corners_match_within_the_corner_square() {
        assert!(matches!(
            zone_at_default(Point::new(3.0, 3.0)),
            Some(window::Direction::NorthWest)
        ));
        assert!(matches!(
            zone_at_default(Point::new(400.0 - 3.0, 3.0)),
            Some(window::Direction::NorthEast)
        ));
        assert!(matches!(
            zone_at_default(Point::new(3.0, 300.0 - 3.0)),
            Some(window::Direction::SouthWest)
        ));
        assert!(matches!(
            zone_at_default(Point::new(400.0 - 3.0, 300.0 - 3.0)),
            Some(window::Direction::SouthEast)
        ));
    }

    #[test]
    fn corners_take_precedence_over_edges() {
        // Both axes are within the corner square but beyond the gutter.
        assert!(matches!(
            zone_at_default(Point::new(12.0, 12.0)),
            Some(window::Direction::NorthWest)
        ));
        assert!(matches!(
            zone_at_default(Point::new(400.0 - 12.0, 300.0 - 12.0)),
            Some(window::Direction::SouthEast)
        ));
    }

    #[test]
    fn corners_are_squares_not_strips() {
        // Within the corner band on one axis, but past the gutter on the
        // other axis and past the corner square: no zone.
        assert!(zone_at_default(Point::new(12.0, 20.0)).is_none());
        assert!(zone_at_default(Point::new(20.0, 12.0)).is_none());
    }

    #[test]
    fn zone_at_respects_bounds_origin() {
        let bounds =
            Rectangle::new(Point::new(100.0, 50.0), Size::new(200.0, 100.0));

        assert!(matches!(
            zone_at(Point::new(102.0, 52.0), bounds, GUTTER, CORNER),
            Some(window::Direction::NorthWest)
        ));
        assert!(zone_at(Point::new(50.0, 50.0), bounds, GUTTER, CORNER)
            .is_none());
    }

    #[test]
    fn interaction_matches_direction() {
        assert!(matches!(
            interaction_for(window::Direction::North),
            Interaction::ResizingVertically
        ));
        assert!(matches!(
            interaction_for(window::Direction::West),
            Interaction::ResizingHorizontally
        ));
        assert!(matches!(
            interaction_for(window::Direction::NorthEast),
            Interaction::ResizingDiagonallyUp
        ));
        assert!(matches!(
            interaction_for(window::Direction::SouthEast),
            Interaction::ResizingDiagonallyDown
        ));
    }
}
