//! The per-frame query layer on top of the input events.

use std::collections::{BTreeMap, BTreeSet};

use super::button::{ButtonState, MouseButton};
use super::event::InputEvent;
use super::ime::ImeEvent;
use super::key::{Key, KeyCode, KeyEvent, Modifiers};
use super::pointer::{PointerPosition, ScrollDelta, TouchPhase, TouchPoint};
use crate::window::WindowEvent;

/// What is held, and what changed this frame, built from input events.
///
/// Events say *what happened*; a game loop asks *what is the state now*. Feed
/// every event of a window to [`handle_event`](Self::handle_event), read the
/// state while updating, and call [`end_frame`](Self::end_frame) once per
/// frame afterwards:
///
/// * `*_pressed` is `true` while the key or button is held;
/// * `*_just_pressed` and `*_just_released` are `true` only in the frame in
///   which the change happened. A key that goes down and up within one frame
///   is reported by both and is not held at the end of it, so a quick tap is
///   never lost.
/// * movement and scrolling add up over the frame.
///
/// Auto-repeat never counts as a new press. When the window loses focus every
/// held key, button and touch is released (and reported as just released), so
/// nothing stays stuck after Alt-Tab.
///
/// One `InputState` follows one window. With several windows keep one per
/// [`WindowId`](crate::window::WindowId), for example in a `BTreeMap`.
///
/// ```
/// use platform::input::{ButtonState, InputEvent, InputState, Key, KeyCode, KeyEvent};
///
/// let mut input = InputState::new();
/// input.handle_input(&InputEvent::Key(KeyEvent::new(
///     KeyCode::KeyW,
///     Key::character("w"),
///     ButtonState::Pressed,
/// )));
/// assert!(input.key_pressed(KeyCode::KeyW) && input.key_just_pressed(KeyCode::KeyW));
///
/// input.end_frame();
/// assert!(input.key_pressed(KeyCode::KeyW) && !input.key_just_pressed(KeyCode::KeyW));
/// ```
#[derive(Debug, Clone, Default)]
pub struct InputState {
    keys: BTreeMap<KeyCode, Key>,
    keys_pressed: BTreeSet<KeyCode>,
    keys_released: BTreeSet<KeyCode>,
    logical_pressed: Vec<Key>,
    logical_released: Vec<Key>,
    buttons: BTreeSet<MouseButton>,
    buttons_pressed: BTreeSet<MouseButton>,
    buttons_released: BTreeSet<MouseButton>,
    modifiers: Modifiers,
    cursor: Option<PointerPosition>,
    cursor_delta: (f64, f64),
    raw_delta: (f64, f64),
    scroll_lines: (f64, f64),
    scroll_pixels: (f64, f64),
    touches: BTreeMap<u64, TouchPoint>,
    touches_started: Vec<TouchPoint>,
    touches_ended: Vec<TouchPoint>,
    text: String,
    preedit: Option<String>,
}

impl InputState {
    /// An empty state: nothing held, nothing happened.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Takes in a window event: input events update the state, losing focus
    /// releases everything, any other event is ignored.
    pub fn handle_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::Input(input) => self.handle_input(input),
            WindowEvent::Focused(false) => self.release_all(),
            _ => {}
        }
    }

    /// Takes in one input event.
    pub fn handle_input(&mut self, event: &InputEvent) {
        match event {
            InputEvent::Key(key) => self.handle_key(key),
            InputEvent::ModifiersChanged(modifiers) => self.modifiers = *modifiers,
            InputEvent::CursorMoved(position) => {
                if let Some(previous) = self.cursor {
                    self.cursor_delta.0 += position.x - previous.x;
                    self.cursor_delta.1 += position.y - previous.y;
                }
                self.cursor = Some(*position);
            }
            // Nothing is known about the cursor until it moves inside the
            // window, and a position from before it left must not produce a
            // jump when it comes back.
            InputEvent::CursorEntered => {}
            InputEvent::CursorLeft => self.cursor = None,
            InputEvent::MouseButton { button, state } => match state {
                ButtonState::Pressed => {
                    if self.buttons.insert(*button) {
                        self.buttons_pressed.insert(*button);
                    }
                }
                ButtonState::Released => {
                    if self.buttons.remove(button) {
                        self.buttons_released.insert(*button);
                    }
                }
            },
            InputEvent::MouseWheel(ScrollDelta::Lines { x, y }) => {
                self.scroll_lines.0 += x;
                self.scroll_lines.1 += y;
            }
            InputEvent::MouseWheel(ScrollDelta::Pixels { x, y }) => {
                self.scroll_pixels.0 += x;
                self.scroll_pixels.1 += y;
            }
            InputEvent::RawMouseMotion { delta_x, delta_y } => {
                self.raw_delta.0 += delta_x;
                self.raw_delta.1 += delta_y;
            }
            InputEvent::Touch(touch) => self.handle_touch(touch.id, touch.phase, touch.position),
            InputEvent::Ime(ime) => match ime {
                ImeEvent::Enabled | ImeEvent::Disabled => self.preedit = None,
                ImeEvent::Preedit { text, .. } => {
                    self.preedit = if text.is_empty() {
                        None
                    } else {
                        Some(text.clone())
                    };
                }
                ImeEvent::Commit(text) => {
                    self.text.push_str(text);
                    self.preedit = None;
                }
            },
        }
    }

    fn handle_key(&mut self, event: &KeyEvent) {
        let code = event.physical_key;
        match event.state {
            ButtonState::Pressed => {
                // A key the platform cannot identify stands for many keys and
                // is not tracked, but the text it types still counts.
                if code != KeyCode::Unidentified && !self.keys.contains_key(&code) {
                    self.keys.insert(code, event.logical_key.clone());
                    self.keys_pressed.insert(code);
                    self.logical_pressed.push(event.logical_key.clone());
                }
                if let Some(text) = &event.text {
                    self.text.extend(text.chars().filter(|c| !c.is_control()));
                }
            }
            ButtonState::Released => {
                if let Some(logical) = self.keys.remove(&code) {
                    self.keys_released.insert(code);
                    self.logical_released.push(logical);
                }
            }
        }
    }

    fn handle_touch(&mut self, id: u64, phase: TouchPhase, position: PointerPosition) {
        match phase {
            TouchPhase::Started => {
                let point = TouchPoint {
                    id,
                    position,
                    start_position: position,
                };
                self.touches.insert(id, point);
                self.touches_started.push(point);
            }
            TouchPhase::Moved => {
                if let Some(point) = self.touches.get_mut(&id) {
                    point.position = position;
                }
            }
            TouchPhase::Ended | TouchPhase::Cancelled => {
                if let Some(mut point) = self.touches.remove(&id) {
                    point.position = position;
                    self.touches_ended.push(point);
                }
            }
        }
    }

    /// Releases everything that is held: keys, mouse buttons, touches and the
    /// modifiers. They show up as just released. Called by
    /// [`handle_event`](Self::handle_event) when the window loses focus.
    pub fn release_all(&mut self) {
        for (code, logical) in std::mem::take(&mut self.keys) {
            self.keys_released.insert(code);
            self.logical_released.push(logical);
        }
        for button in std::mem::take(&mut self.buttons) {
            self.buttons_released.insert(button);
        }
        self.touches_ended
            .extend(std::mem::take(&mut self.touches).into_values());
        self.modifiers = Modifiers::NONE;
        self.preedit = None;
    }

    /// Ends the frame: forgets what only lasts one frame (just pressed, just
    /// released, movement, scrolling, typed text, started and ended touches).
    /// What is held, the cursor position, the modifiers and the preedit text
    /// stay.
    pub fn end_frame(&mut self) {
        self.keys_pressed.clear();
        self.keys_released.clear();
        self.logical_pressed.clear();
        self.logical_released.clear();
        self.buttons_pressed.clear();
        self.buttons_released.clear();
        self.cursor_delta = (0.0, 0.0);
        self.raw_delta = (0.0, 0.0);
        self.scroll_lines = (0.0, 0.0);
        self.scroll_pixels = (0.0, 0.0);
        self.touches_started.clear();
        self.touches_ended.clear();
        self.text.clear();
    }

    // Physical keys.

    /// `true` while the physical key is held.
    #[must_use]
    pub fn key_pressed(&self, key: KeyCode) -> bool {
        self.keys.contains_key(&key)
    }

    /// `true` in the frame in which the physical key went down.
    #[must_use]
    pub fn key_just_pressed(&self, key: KeyCode) -> bool {
        self.keys_pressed.contains(&key)
    }

    /// `true` in the frame in which the physical key went up.
    #[must_use]
    pub fn key_just_released(&self, key: KeyCode) -> bool {
        self.keys_released.contains(&key)
    }

    /// The held physical keys, in the order of [`KeyCode`].
    pub fn pressed_keys(&self) -> impl Iterator<Item = KeyCode> + '_ {
        self.keys.keys().copied()
    }

    /// The physical keys that went down this frame.
    pub fn just_pressed_keys(&self) -> impl Iterator<Item = KeyCode> + '_ {
        self.keys_pressed.iter().copied()
    }

    /// The physical keys that went up this frame.
    pub fn just_released_keys(&self) -> impl Iterator<Item = KeyCode> + '_ {
        self.keys_released.iter().copied()
    }

    // Logical keys.

    /// `true` while a key with this logical meaning is held. The meaning is
    /// the one the key had when it went down, so releasing Shift before the
    /// letter does not make `"A"` turn into `"a"` in between.
    #[must_use]
    pub fn logical_key_pressed(&self, key: &Key) -> bool {
        self.keys.values().any(|held| held == key)
    }

    /// `true` in the frame in which a key with this logical meaning went down.
    #[must_use]
    pub fn logical_key_just_pressed(&self, key: &Key) -> bool {
        self.logical_pressed.contains(key)
    }

    /// `true` in the frame in which a key with this logical meaning went up.
    #[must_use]
    pub fn logical_key_just_released(&self, key: &Key) -> bool {
        self.logical_released.contains(key)
    }

    /// The modifier keys that are held.
    #[must_use]
    pub fn modifiers(&self) -> Modifiers {
        self.modifiers
    }

    // Mouse.

    /// `true` while the mouse button is held.
    #[must_use]
    pub fn mouse_pressed(&self, button: MouseButton) -> bool {
        self.buttons.contains(&button)
    }

    /// `true` in the frame in which the mouse button went down.
    #[must_use]
    pub fn mouse_just_pressed(&self, button: MouseButton) -> bool {
        self.buttons_pressed.contains(&button)
    }

    /// `true` in the frame in which the mouse button went up.
    #[must_use]
    pub fn mouse_just_released(&self, button: MouseButton) -> bool {
        self.buttons_released.contains(&button)
    }

    /// The held mouse buttons.
    pub fn pressed_mouse_buttons(&self) -> impl Iterator<Item = MouseButton> + '_ {
        self.buttons.iter().copied()
    }

    /// Where the cursor is in the window (physical pixels), or `None` while it
    /// is outside the window or has not moved inside it yet.
    #[must_use]
    pub fn cursor_position(&self) -> Option<PointerPosition> {
        self.cursor
    }

    /// How far the cursor moved this frame, as `(x, y)` in physical pixels.
    /// Moves while the cursor was outside the window do not count.
    #[must_use]
    pub fn cursor_delta(&self) -> (f64, f64) {
        self.cursor_delta
    }

    /// The raw mouse movement of this frame, `(x, y)` in device units. Use it
    /// for camera control; it also works while the cursor is locked.
    #[must_use]
    pub fn raw_mouse_delta(&self) -> (f64, f64) {
        self.raw_delta
    }

    /// The scrolling of this frame in lines (notched wheels), `(x, y)`.
    #[must_use]
    pub fn scroll_lines(&self) -> (f64, f64) {
        self.scroll_lines
    }

    /// The scrolling of this frame in pixels (touchpads), `(x, y)`.
    #[must_use]
    pub fn scroll_pixels(&self) -> (f64, f64) {
        self.scroll_pixels
    }

    // Touch.

    /// The fingers that are on the screen, ordered by id.
    pub fn touches(&self) -> impl Iterator<Item = &TouchPoint> {
        self.touches.values()
    }

    /// The finger with this id, if it is on the screen.
    #[must_use]
    pub fn touch(&self, id: u64) -> Option<&TouchPoint> {
        self.touches.get(&id)
    }

    /// The touches that started this frame.
    #[must_use]
    pub fn touches_started(&self) -> &[TouchPoint] {
        &self.touches_started
    }

    /// The touches that ended this frame, cancelled ones included, with the
    /// position at which they ended.
    #[must_use]
    pub fn touches_ended(&self) -> &[TouchPoint] {
        &self.touches_ended
    }

    // Text.

    /// The text typed this frame: printable characters of key presses (with
    /// the layout and Shift applied, auto-repeat included) followed by what an
    /// input method committed. Control characters are left out; react to Enter
    /// or Backspace through the keys.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The text an input method is composing right now, if any. Unlike
    /// [`text`](Self::text) it lasts until the composition changes.
    #[must_use]
    pub fn ime_preedit(&self) -> Option<&str> {
        self.preedit.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::super::key::NamedKey;
    use super::super::pointer::TouchEvent;
    use super::*;

    fn key(code: KeyCode, logical: Key, state: ButtonState) -> InputEvent {
        InputEvent::Key(KeyEvent::new(code, logical, state))
    }

    fn down(code: KeyCode) -> InputEvent {
        key(code, Key::character("x"), ButtonState::Pressed)
    }

    fn up(code: KeyCode) -> InputEvent {
        key(code, Key::character("x"), ButtonState::Released)
    }

    fn mouse(button: MouseButton, state: ButtonState) -> InputEvent {
        InputEvent::MouseButton { button, state }
    }

    fn moved(x: f64, y: f64) -> InputEvent {
        InputEvent::CursorMoved(PointerPosition::new(x, y))
    }

    fn touch(id: u64, phase: TouchPhase, x: f64, y: f64) -> InputEvent {
        InputEvent::Touch(TouchEvent::new(id, phase, PointerPosition::new(x, y)))
    }

    #[test]
    fn a_new_state_is_empty() {
        let s = InputState::new();
        assert_eq!(s.pressed_keys().count(), 0);
        assert!(!s.key_pressed(KeyCode::KeyA));
        assert!(s.cursor_position().is_none());
        assert_eq!(s.cursor_delta(), (0.0, 0.0));
        assert_eq!(s.text(), "");
        assert!(s.modifiers().is_empty());
        assert!(s.ime_preedit().is_none());
        assert_eq!(s.touches().count(), 0);
    }

    #[test]
    fn a_key_is_held_just_pressed_then_just_released() {
        let mut s = InputState::new();
        s.handle_input(&down(KeyCode::KeyW));
        assert!(s.key_pressed(KeyCode::KeyW));
        assert!(s.key_just_pressed(KeyCode::KeyW));
        assert!(!s.key_just_released(KeyCode::KeyW));
        assert_eq!(s.just_pressed_keys().collect::<Vec<_>>(), [KeyCode::KeyW]);

        s.end_frame();
        assert!(s.key_pressed(KeyCode::KeyW));
        assert!(!s.key_just_pressed(KeyCode::KeyW));

        s.handle_input(&up(KeyCode::KeyW));
        assert!(!s.key_pressed(KeyCode::KeyW));
        assert!(s.key_just_released(KeyCode::KeyW));
        assert_eq!(s.just_released_keys().collect::<Vec<_>>(), [KeyCode::KeyW]);

        s.end_frame();
        assert!(!s.key_just_released(KeyCode::KeyW));
    }

    #[test]
    fn a_tap_within_one_frame_is_not_lost() {
        let mut s = InputState::new();
        s.handle_input(&down(KeyCode::Space));
        s.handle_input(&up(KeyCode::Space));
        assert!(!s.key_pressed(KeyCode::Space));
        assert!(s.key_just_pressed(KeyCode::Space));
        assert!(s.key_just_released(KeyCode::Space));
    }

    #[test]
    fn auto_repeat_is_not_a_new_press() {
        let mut s = InputState::new();
        s.handle_input(&down(KeyCode::KeyA));
        s.end_frame();
        let repeat = InputEvent::Key(
            KeyEvent::new(KeyCode::KeyA, Key::character("a"), ButtonState::Pressed)
                .with_repeat(true),
        );
        s.handle_input(&repeat);
        assert!(s.key_pressed(KeyCode::KeyA));
        assert!(!s.key_just_pressed(KeyCode::KeyA));
    }

    #[test]
    fn a_release_without_a_press_is_ignored() {
        let mut s = InputState::new();
        s.handle_input(&up(KeyCode::KeyA));
        assert!(!s.key_just_released(KeyCode::KeyA));
        s.handle_input(&mouse(MouseButton::Left, ButtonState::Released));
        assert!(!s.mouse_just_released(MouseButton::Left));
    }

    #[test]
    fn unidentified_keys_are_not_tracked_but_their_text_counts() {
        let mut s = InputState::new();
        let event = KeyEvent::new(
            KeyCode::Unidentified,
            Key::Unidentified,
            ButtonState::Pressed,
        )
            .with_text("é");
        s.handle_input(&InputEvent::Key(event));
        assert_eq!(s.pressed_keys().count(), 0);
        assert_eq!(s.text(), "é");
    }

    #[test]
    fn keys_are_listed_in_code_order() {
        let mut s = InputState::new();
        s.handle_input(&down(KeyCode::KeyS));
        s.handle_input(&down(KeyCode::KeyA));
        assert_eq!(
            s.pressed_keys().collect::<Vec<_>>(),
            [KeyCode::KeyA, KeyCode::KeyS]
        );
    }

    #[test]
    fn logical_keys_keep_the_meaning_they_had_when_pressed() {
        let mut s = InputState::new();
        let upper = Key::character("A");
        s.handle_input(&key(KeyCode::KeyA, upper.clone(), ButtonState::Pressed));
        assert!(s.logical_key_pressed(&upper));
        assert!(s.logical_key_just_pressed(&upper));
        assert!(!s.logical_key_pressed(&Key::character("a")));
        // Shift went up first, so the release reports the lower-case letter.
        s.handle_input(&key(
            KeyCode::KeyA,
            Key::character("a"),
            ButtonState::Released,
        ));
        assert!(!s.logical_key_pressed(&upper));
        assert!(s.logical_key_just_released(&upper));
        assert!(!s.logical_key_just_released(&Key::character("a")));
    }

    #[test]
    fn named_keys_can_be_queried_logically() {
        let mut s = InputState::new();
        let enter = Key::Named(NamedKey::Enter);
        s.handle_input(&key(KeyCode::Enter, enter.clone(), ButtonState::Pressed));
        assert!(s.logical_key_just_pressed(&enter));
    }

    #[test]
    fn mouse_buttons_follow_the_same_rules() {
        let mut s = InputState::new();
        s.handle_input(&mouse(MouseButton::Left, ButtonState::Pressed));
        s.handle_input(&mouse(MouseButton::Other(7), ButtonState::Pressed));
        assert!(s.mouse_pressed(MouseButton::Left) && s.mouse_just_pressed(MouseButton::Left));
        assert_eq!(
            s.pressed_mouse_buttons().collect::<Vec<_>>(),
            [MouseButton::Left, MouseButton::Other(7)]
        );
        s.end_frame();
        s.handle_input(&mouse(MouseButton::Left, ButtonState::Pressed));
        assert!(!s.mouse_just_pressed(MouseButton::Left), "already held");
        s.handle_input(&mouse(MouseButton::Left, ButtonState::Released));
        assert!(!s.mouse_pressed(MouseButton::Left) && s.mouse_just_released(MouseButton::Left));
    }

    #[test]
    fn the_cursor_delta_adds_up_and_resets_each_frame() {
        let mut s = InputState::new();
        s.handle_input(&moved(10.0, 10.0));
        assert_eq!(
            s.cursor_delta(),
            (0.0, 0.0),
            "the first position has no delta"
        );
        s.handle_input(&moved(13.0, 9.0));
        s.handle_input(&moved(14.0, 12.0));
        assert_eq!(s.cursor_position(), Some(PointerPosition::new(14.0, 12.0)));
        assert_eq!(s.cursor_delta(), (4.0, 2.0));
        s.end_frame();
        assert_eq!(s.cursor_delta(), (0.0, 0.0));
        assert_eq!(s.cursor_position(), Some(PointerPosition::new(14.0, 12.0)));
    }

    #[test]
    fn leaving_the_window_forgets_the_position_and_avoids_a_jump_on_return() {
        let mut s = InputState::new();
        s.handle_input(&moved(10.0, 10.0));
        s.handle_input(&InputEvent::CursorLeft);
        assert!(s.cursor_position().is_none());
        s.handle_input(&InputEvent::CursorEntered);
        s.handle_input(&moved(500.0, 400.0));
        assert_eq!(s.cursor_delta(), (0.0, 0.0));
        assert_eq!(
            s.cursor_position(),
            Some(PointerPosition::new(500.0, 400.0))
        );
    }

    #[test]
    fn raw_motion_and_scrolling_accumulate_separately() {
        let mut s = InputState::new();
        s.handle_input(&InputEvent::RawMouseMotion {
            delta_x: 2.0,
            delta_y: -1.0,
        });
        s.handle_input(&InputEvent::RawMouseMotion {
            delta_x: 3.0,
            delta_y: 1.5,
        });
        s.handle_input(&InputEvent::MouseWheel(ScrollDelta::Lines {
            x: 0.0,
            y: 1.0,
        }));
        s.handle_input(&InputEvent::MouseWheel(ScrollDelta::Lines {
            x: 0.5,
            y: 2.0,
        }));
        s.handle_input(&InputEvent::MouseWheel(ScrollDelta::Pixels {
            x: 10.0,
            y: -4.0,
        }));
        assert_eq!(s.raw_mouse_delta(), (5.0, 0.5));
        assert_eq!(s.scroll_lines(), (0.5, 3.0));
        assert_eq!(s.scroll_pixels(), (10.0, -4.0));
        s.end_frame();
        assert_eq!(s.raw_mouse_delta(), (0.0, 0.0));
        assert_eq!(s.scroll_lines(), (0.0, 0.0));
        assert_eq!(s.scroll_pixels(), (0.0, 0.0));
    }

    #[test]
    fn modifiers_follow_the_events_and_survive_the_frame() {
        let mut s = InputState::new();
        s.handle_input(&InputEvent::ModifiersChanged(
            Modifiers::CONTROL | Modifiers::SHIFT,
        ));
        assert!(s.modifiers().control() && s.modifiers().shift());
        s.end_frame();
        assert!(s.modifiers().control());
        s.handle_input(&InputEvent::ModifiersChanged(Modifiers::NONE));
        assert!(s.modifiers().is_empty());
    }

    #[test]
    fn touches_are_tracked_from_start_to_end() {
        let mut s = InputState::new();
        s.handle_input(&touch(1, TouchPhase::Started, 5.0, 6.0));
        s.handle_input(&touch(2, TouchPhase::Started, 50.0, 60.0));
        assert_eq!(s.touches().map(|t| t.id).collect::<Vec<_>>(), [1, 2]);
        assert_eq!(s.touches_started().len(), 2);

        s.end_frame();
        assert!(s.touches_started().is_empty());
        s.handle_input(&touch(1, TouchPhase::Moved, 8.0, 9.0));
        let point = s.touch(1).unwrap();
        assert_eq!(point.position, PointerPosition::new(8.0, 9.0));
        assert_eq!(point.start_position, PointerPosition::new(5.0, 6.0));

        s.handle_input(&touch(1, TouchPhase::Ended, 8.5, 9.5));
        s.handle_input(&touch(2, TouchPhase::Cancelled, 50.0, 61.0));
        assert_eq!(s.touches().count(), 0);
        let ended: Vec<_> = s
            .touches_ended()
            .iter()
            .map(|t| (t.id, t.position))
            .collect();
        assert_eq!(
            ended,
            [
                (1, PointerPosition::new(8.5, 9.5)),
                (2, PointerPosition::new(50.0, 61.0))
            ]
        );
        s.end_frame();
        assert!(s.touches_ended().is_empty());
    }

    #[test]
    fn a_move_or_end_of_an_unknown_touch_is_ignored() {
        let mut s = InputState::new();
        s.handle_input(&touch(9, TouchPhase::Moved, 1.0, 1.0));
        s.handle_input(&touch(9, TouchPhase::Ended, 1.0, 1.0));
        assert_eq!(s.touches().count(), 0);
        assert!(s.touches_ended().is_empty());
    }

    #[test]
    fn typed_text_collects_printable_characters_only() {
        let mut s = InputState::new();
        for (code, text) in [
            (KeyCode::KeyH, "H"),
            (KeyCode::KeyI, "i"),
            (KeyCode::Enter, "\r"),
            (KeyCode::Backspace, "\u{8}"),
            (KeyCode::Tab, "\t"),
        ] {
            let event =
                KeyEvent::new(code, Key::character(text), ButtonState::Pressed).with_text(text);
            s.handle_input(&InputEvent::Key(event));
        }
        assert_eq!(s.text(), "Hi");
        s.end_frame();
        assert_eq!(s.text(), "");
    }

    #[test]
    fn text_from_auto_repeat_is_kept() {
        let mut s = InputState::new();
        for repeat in [false, true, true] {
            let event = KeyEvent::new(KeyCode::KeyA, Key::character("a"), ButtonState::Pressed)
                .with_text("a")
                .with_repeat(repeat);
            s.handle_input(&InputEvent::Key(event));
        }
        assert_eq!(s.text(), "aaa");
    }

    #[test]
    fn ime_composition_updates_the_preedit_and_commits_text() {
        let mut s = InputState::new();
        s.handle_input(&InputEvent::Ime(ImeEvent::Enabled));
        s.handle_input(&InputEvent::Ime(ImeEvent::Preedit {
            text: "にほ".to_owned(),
            cursor: Some((0, 6)),
        }));
        assert_eq!(s.ime_preedit(), Some("にほ"));
        s.end_frame();
        assert_eq!(
            s.ime_preedit(),
            Some("にほ"),
            "the preedit outlives the frame"
        );

        s.handle_input(&InputEvent::Ime(ImeEvent::Commit("日本".to_owned())));
        assert_eq!(s.text(), "日本");
        assert!(s.ime_preedit().is_none());

        s.handle_input(&InputEvent::Ime(ImeEvent::Preedit {
            text: "x".to_owned(),
            cursor: None,
        }));
        s.handle_input(&InputEvent::Ime(ImeEvent::Preedit {
            text: String::new(),
            cursor: None,
        }));
        assert!(s.ime_preedit().is_none(), "an empty preedit clears it");
        s.handle_input(&InputEvent::Ime(ImeEvent::Preedit {
            text: "y".to_owned(),
            cursor: None,
        }));
        s.handle_input(&InputEvent::Ime(ImeEvent::Disabled));
        assert!(s.ime_preedit().is_none());
    }

    #[test]
    fn losing_focus_releases_everything() {
        let mut s = InputState::new();
        s.handle_input(&down(KeyCode::KeyW));
        s.handle_input(&mouse(MouseButton::Right, ButtonState::Pressed));
        s.handle_input(&touch(3, TouchPhase::Started, 1.0, 2.0));
        s.handle_input(&InputEvent::ModifiersChanged(Modifiers::ALT));
        s.end_frame();

        s.handle_event(&WindowEvent::Focused(false));
        assert!(!s.key_pressed(KeyCode::KeyW) && s.key_just_released(KeyCode::KeyW));
        assert!(!s.mouse_pressed(MouseButton::Right) && s.mouse_just_released(MouseButton::Right));
        assert_eq!(s.touches().count(), 0);
        assert_eq!(s.touches_ended().len(), 1);
        assert!(s.modifiers().is_empty());
        assert!(s.logical_key_just_released(&Key::character("x")));
    }

    #[test]
    fn gaining_focus_and_unrelated_window_events_change_nothing() {
        let mut s = InputState::new();
        s.handle_input(&down(KeyCode::KeyW));
        s.handle_event(&WindowEvent::Focused(true));
        s.handle_event(&WindowEvent::RedrawRequested);
        s.handle_event(&WindowEvent::CloseRequested);
        assert!(s.key_pressed(KeyCode::KeyW));
        assert!(!s.key_just_released(KeyCode::KeyW));
    }

    #[test]
    fn window_input_events_reach_the_state() {
        let mut s = InputState::new();
        s.handle_event(&WindowEvent::Input(down(KeyCode::KeyQ)));
        assert!(s.key_pressed(KeyCode::KeyQ));
    }
}
