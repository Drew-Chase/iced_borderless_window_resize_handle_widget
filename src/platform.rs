//! Platform glue for starting native window resize drags.
//!
//! # The Windows cursor quirk
//!
//! Entering the native size loop (`WM_NCLBUTTONDOWN` with an `HT*` hit-test
//! value) suspends all `WM_SETCURSOR` traffic for the duration of the drag:
//! the modal loop captures the mouse, delivers no cursor messages (not even
//! *sent* ones are serviced), and winit deliberately registers its window
//! class with a `NULL` class cursor. When the cursor slips a pixel past the
//! moving window edge for a frame, the system switches it to the desktop's
//! default arrow — and nothing ever switches it back.
//!
//! Iced is suspended inside the modal loop and cannot fix this itself, so
//! [`drag_resize`] spawns a detached thread that re-asserts the matching OS
//! resize cursor while the left mouse button is held. Because a cursor set
//! from a foreign thread is ignored while the loop owns capture, the thread
//! first attaches its input queue to the window's thread with
//! [`AttachThreadInput`]; and because `SetCursor` ignores a value equal to
//! the queue's last known cursor, it clears the cursor before re-applying
//! it. On every other platform the native resize machinery manages the
//! cursor correctly and [`drag_resize`] is a plain pass-through.
//!
//! [`AttachThreadInput`]: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-attachthreadinput

/// Begins resizing the window while the left mouse button is held — a
/// drop-in replacement for [`iced::window::drag_resize`] that keeps the
/// native resize cursor visible for the whole drag on Windows.
pub fn drag_resize<T>(id: iced::window::Id, direction: iced::window::Direction) -> iced::Task<T> {
    imp::drag_resize(id, direction)
}

#[cfg(target_os = "windows")]
mod imp {
    use iced::window::{self, Direction};
    use iced::Task;

    use std::time::{Duration, Instant};

    pub fn drag_resize<T>(id: window::Id, direction: Direction) -> Task<T> {
        spawn_cursor_keeper(direction);

        window::drag_resize(id, direction)
    }

    /// Asserts the matching OS resize cursor while the left mouse button is
    /// held.
    ///
    /// Runs on a detached thread: the runtime is frozen inside the modal
    /// size loop and delivers no events until the button is released. The
    /// loop exits as soon as the button comes up, at which point normal
    /// cursor handling resumes.
    fn spawn_cursor_keeper(direction: Direction) {
        std::thread::spawn(move || {
            // The window under the press point is the window being resized.
            let mut point = POINT { x: 0, y: 0 };
            unsafe { GetCursorPos(&mut point) };

            let hwnd = unsafe { GetAncestor(WindowFromPoint(point), GA_ROOT) };
            let mut pid = 0u32;
            let window_thread = unsafe { GetWindowThreadProcessId(hwnd, &mut pid) };

            if hwnd == 0 || pid != std::process::id() {
                return;
            }

            let hcursor = cursor_for(direction);
            let keeper_thread = unsafe { GetCurrentThreadId() };

            // The modal loop owns mouse capture on the window's thread, and a
            // `SetCursor` call from any other queue is ignored. Attaching the
            // input queues lets this thread drive the shared cursor.
            unsafe { AttachThreadInput(keeper_thread, window_thread, 1) };

            // The deadline is a safety net against a stuck key state keeping
            // this thread alive forever.
            let started = Instant::now();
            while started.elapsed() < Duration::from_secs(60)
                && unsafe { GetAsyncKeyState(VK_LBUTTON.into()) } < 0
            {
                // `SetCursor` ignores values equal to the queue's last known
                // cursor — which is still the resize cursor from the hover,
                // even though the modal loop switched the visible cursor to
                // the desktop's arrow. Clearing the cursor first forces the
                // change through.
                unsafe {
                    SetCursor(0);
                    SetCursor(hcursor);
                };
                std::thread::sleep(Duration::from_millis(15));
            }

            unsafe { AttachThreadInput(keeper_thread, window_thread, 0) };
        });
    }

    fn cursor_for(direction: Direction) -> HCURSOR {
        let cursor = match direction {
            Direction::North | Direction::South => IDC_SIZENS,
            Direction::East | Direction::West => IDC_SIZEWE,
            Direction::NorthEast | Direction::SouthWest => IDC_SIZENESW,
            Direction::NorthWest | Direction::SouthEast => IDC_SIZENWSE,
        };

        unsafe { LoadCursorW(0, cursor) }
    }

    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::System::Threading::{
        AttachThreadInput, GetCurrentThreadId,
    };
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, VK_LBUTTON,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetAncestor, GetCursorPos, GetWindowThreadProcessId, LoadCursorW, SetCursor,
        WindowFromPoint, GA_ROOT, HCURSOR, IDC_SIZENESW, IDC_SIZENS, IDC_SIZENWSE, IDC_SIZEWE,
    };
}

#[cfg(not(target_os = "windows"))]
mod imp {
    pub fn drag_resize<T>(
        id: iced::window::Id,
        direction: iced::window::Direction,
    ) -> iced::Task<T> {
        iced::window::drag_resize(id, direction)
    }
}
