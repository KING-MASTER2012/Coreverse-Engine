//! Input: keyboard, mouse, touch and input-method events, and a per-frame
//! query layer on top of them.
//!
//! Input reaches an application as [`WindowEvent::Input`](crate::window::WindowEvent::Input),
//! carrying an [`InputEvent`]. Every [`Backend`](crate::window::Backend)
//! delivers the same events, so game code does not change between the
//! headless backend (where a test plays the user with
//! [`HostEvent::Input`](crate::window::HostEvent::Input)), the desktop backend
//! and, later, the mobile ones.
//!
//! # Events
//!
//! * **Keyboard** — [`KeyEvent`] with both identities of a key: the physical
//!   [`KeyCode`] (where it sits, for controls such as WASD) and the logical
//!   [`Key`] (what it means under the layout, for shortcuts and text), the
//!   text it produces, whether it is an auto-repeat, and
//!   [`Modifiers`] changes.
//! * **Mouse** — cursor movement, enter and leave, buttons, wheel, and *raw*
//!   motion for camera control that keeps working while the cursor is locked.
//! * **Touch** — [`TouchEvent`] per finger, with
//!   [`Started`](TouchPhase::Started), [`Moved`](TouchPhase::Moved),
//!   [`Ended`](TouchPhase::Ended) and [`Cancelled`](TouchPhase::Cancelled).
//! * **IME** — [`ImeEvent`]: composition text and the text it commits. The
//!   window must allow it first
//!   ([`AppContext::set_ime_allowed`](crate::window::AppContext::set_ime_allowed)).
//!
//! # Querying state
//!
//! [`InputState`] turns the event stream into state: held keys and buttons,
//! `just_pressed` / `just_released` for the current frame, cursor, movement
//! and scroll deltas, touches and typed text.
//!
//! ```
//! use platform::input::{InputState, KeyCode, MouseButton};
//! use platform::window::{App, AppContext, WindowEvent, WindowId};
//!
//! #[derive(Default)]
//! struct Game {
//!     input: InputState,
//!     jumps: u32,
//! }
//!
//! impl App for Game {
//!     fn window_event(&mut self, _ctx: &mut AppContext, _window: WindowId, event: WindowEvent) {
//!         self.input.handle_event(&event); // collect
//!     }
//!
//!     fn about_to_wait(&mut self, _ctx: &mut AppContext) {
//!         // update: ask, don't track
//!         if self.input.key_just_pressed(KeyCode::Space) {
//!             self.jumps += 1;
//!         }
//!         let _firing = self.input.mouse_pressed(MouseButton::Left);
//!         self.input.end_frame(); // forget what only lasts one frame
//!     }
//! }
//! ```
//!
//! # Platform notes
//!
//! | Input | Windows, macOS, Linux (X11, Wayland) | Android, iOS |
//! | :---- | :----------------------------------: | :----------: |
//! | Keyboard, mouse, modifiers | yes | keyboard only with an attached keyboard |
//! | Raw mouse motion | yes (Wayland: with the relative-pointer protocol) | no |
//! | Touch | where the system sends touch events (Windows, X11 with XInput2, Wayland) | yes, backend planned |
//! | IME | yes, after `set_ime_allowed(true)` | planned with the mobile backends |
//!
//! Only the headless backend and the X11 desktop backend (under Xvfb) have
//! been run by the project's own checks; see the platform chapter of the book
//! for what was verified where.

mod button;
mod event;
mod ime;
mod key;
mod pointer;
mod state;

pub use button::{ButtonState, MouseButton};
pub use event::InputEvent;
pub use ime::ImeEvent;
pub use key::{Key, KeyCode, KeyEvent, Modifiers, NamedKey};
pub use pointer::{PointerPosition, ScrollDelta, TouchEvent, TouchPhase, TouchPoint};
pub use state::InputState;
