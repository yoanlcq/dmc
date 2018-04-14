//! Events reported by the platform.

use std::time::Duration;
use timeout::Timeout;
use super::{Vec2, Extent2, Rect};
use context::Context;
use error;
use window::WindowHandle;
use hid::*;

/// A platform-specific timestamp starting from an unspecified instant.
///
/// This type should be an alias to `Instant` instead, but the only way to create an `Instant` is
/// via the `now()` associated function.
pub type Timestamp = Duration;

impl Context {
    /// Are raw device events supported ? (i.e `MouseMotionRaw`,
    /// `KeyboardKeyPressedRaw`, etc).
    pub fn supports_raw_device_events(&self) -> error::Result<bool> {
        self.0.supports_raw_device_events()
    }
    /// Polls for any event in the queue.
    pub fn poll_event(&self) -> Option<Event> {
        self.events_poll_iter().next()
    }
    /// Waits for any event in the queue.
    pub fn wait_event(&self, timeout: Timeout) -> Option<Event> {
        self.events_wait_iter(timeout).next()
    }
    /// Returns an iterator that polls for events in the queue.
    pub fn events_poll_iter(&self) -> Iter {
        self.events_wait_iter(Timeout::none())
    }
    /// Returns an iterator that waits for events in the queue.
    pub fn events_wait_iter(&self, timeout: Timeout) -> Iter {
        Iter { context: self, timeout }
    }
}

/// An iterator that yields events, removing them from the system queue.
#[derive(Debug)]
pub struct Iter<'c> {
    pub(crate) context: &'c Context,
    pub(crate) timeout: Timeout,
}

impl<'c> Iterator for Iter<'c> {
    type Item = Event;
    fn next(&mut self) -> Option<Event> {
        self.context.0.next_event(self.timeout)
    }
}

// TODO Missing event types:
// - Drag'n drop
// - OpenGL context loss
// - Screen plugged/unplugged
// - Audio device plugged/unplugged
// - Trackball features for the mouse.
// - Missing 'timestamp' field for most events

#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// Quit requested. See https://wiki.libsdl.org/SDL_EventType#SDL_QUIT
    Quit,
    // Mobile events: See SDL_APP_* events
    AppBeingTerminatedByOS,
    AppLowMemory,
    AppWillEnterBackground,
    AppEnteredBackground,
    AppWillEnterForeground,
    AppEnteredForeground,

    /// Asks "Is it OK to terminate you ?". (Windows: WM_QUERYENDSESSION)
    SessionEndRequested,
    /// Perform clean-up operations here. (Window: WM_ENDSESSION)
    SessionEnding,

    TextInput { string: String, timestamp: Timestamp, },

    // 
    // Window events
    //
    WindowShown          { window: WindowHandle, },
    WindowHidden         { window: WindowHandle, },
    WindowNeedsRedraw    { window: WindowHandle, zone: Rect<u32, u32>, more_to_follow: usize, },
    WindowMoved          { window: WindowHandle, position: Vec2<i32>, by_user: bool, },
    WindowResized        { window: WindowHandle, size: Extent2<u32>, by_user: bool, },
    WindowMinimized      { window: WindowHandle, },
    WindowMaximized      { window: WindowHandle, },
    WindowUnminized      { window: WindowHandle, }, // XXX
    WindowCloseRequested { window: WindowHandle, },
    // NOTE: A lot of other window events missing

    //
    // HIDs
    //

    // Audio. NOTE: Missing fields.
    AudioOutputDeviceConnected,
    AudioOutputDeviceDisconnected,
    AudioCaptureDeviceConnected,
    AudioCaptureDeviceDisconnected,

    // User note: in MouseScroll, the y value is positive when "scrolling up"
    // (that is, pushing the wheel forwards) and negative otherwise.
    MouseConnected         { mouse: MouseId, timestamp: Timestamp, },
    MouseDisconnected      { mouse: MouseId, timestamp: Timestamp, },
    MouseEnter             { mouse: MouseId, window: WindowHandle, timestamp: Timestamp, position: Vec2<f64>, root_position: Vec2<f64>, is_grabbed: bool,  is_focused: bool, },
    MouseLeave             { mouse: MouseId, window: WindowHandle, timestamp: Timestamp, position: Vec2<f64>, root_position: Vec2<f64>, was_grabbed: bool, was_focused: bool, },
    MouseButtonPressed     { mouse: MouseId, window: WindowHandle, timestamp: Timestamp, position: Vec2<f64>, root_position: Vec2<f64>, button: MouseButton, clicks: Option<u32>, },
    MouseButtonReleased    { mouse: MouseId, window: WindowHandle, timestamp: Timestamp, position: Vec2<f64>, root_position: Vec2<f64>, button: MouseButton, },
    MouseScroll            { mouse: MouseId, window: WindowHandle, timestamp: Timestamp, position: Vec2<f64>, root_position: Vec2<f64>, scroll: Vec2<i32>, },
    MouseMotion            { mouse: MouseId, window: WindowHandle, timestamp: Timestamp, position: Vec2<f64>, root_position: Vec2<f64>, },
    MouseButtonPressedRaw  { mouse: MouseId, timestamp: Timestamp, button: MouseButton, },
    MouseButtonReleasedRaw { mouse: MouseId, timestamp: Timestamp, button: MouseButton, },
    MouseScrollRaw         { mouse: MouseId, timestamp: Timestamp, scroll: Vec2<i32>, },
    MouseMotionRaw         { mouse: MouseId, timestamp: Timestamp, displacement: Vec2<f64>, },

    // Keyboard
    KeyboardConnected      { keyboard: KeyboardId, timestamp: Timestamp, },
    KeyboardDisconnected   { keyboard: KeyboardId, timestamp: Timestamp, },
    KeyboardFocusGained    { keyboard: KeyboardId, window: WindowHandle, timestamp: Timestamp, },
    KeyboardFocusLost      { keyboard: KeyboardId, window: WindowHandle, timestamp: Timestamp, },
    KeyboardKeyPressed     { keyboard: KeyboardId, window: WindowHandle, timestamp: Timestamp, key: Key, is_repeat: bool, text: Option<String>, },
    KeyboardKeyReleased    { keyboard: KeyboardId, window: WindowHandle, timestamp: Timestamp, key: Key, },
    KeyboardKeyPressedRaw  { keyboard: KeyboardId, timestamp: Timestamp, key: Key, },
    KeyboardKeyReleasedRaw { keyboard: KeyboardId, timestamp: Timestamp, key: Key, },

    // Touch (Touchpad, Touch-screen, ....)
    TouchConnected      { touch: TouchId, timestamp: Timestamp, },
    TouchDisconnected   { touch: TouchId, timestamp: Timestamp, },
    TouchFingerPressed  { touch: TouchId, timestamp: Timestamp, finger: u32, pressure: f64, normalized_position: Vec2<f64>, },
    TouchFingerReleased { touch: TouchId, timestamp: Timestamp, finger: u32, pressure: f64, normalized_position: Vec2<f64>, },
    TouchFingerMotion   { touch: TouchId, timestamp: Timestamp, finger: u32, pressure: f64, normalized_motion:   Vec2<f64>, },
    TouchMultiGesture   { touch: TouchId, timestamp: Timestamp, nb_fingers: usize, rotation_radians: f64, pinch: f64, normalized_center: Vec2<f64>, },
    // NOTE: Missing raw events

    TabletConnected               { tablet: TabletId, timestamp: Timestamp, },
    TabletDisconnected            { tablet: TabletId, timestamp: Timestamp, },
    TabletPadButtonPressed        { tablet: TabletId, timestamp: Timestamp, window: WindowHandle, button: TabletPadButton, },
    TabletPadButtonReleased       { tablet: TabletId, timestamp: Timestamp, window: WindowHandle, button: TabletPadButton, },
    TabletStylusToolType          { tablet: TabletId, timestamp: Timestamp, window: WindowHandle, position: Vec2<f64>, root_position: Vec2<f64>, tool_type: TabletStylusToolType, },
    TabletStylusButtonPressed     { tablet: TabletId, timestamp: Timestamp, window: WindowHandle, position: Vec2<f64>, root_position: Vec2<f64>, pressure: f64, tilt: Vec2<f64>, physical_position: Vec2<f64>, button: TabletStylusButton, },
    TabletStylusButtonReleased    { tablet: TabletId, timestamp: Timestamp, window: WindowHandle, position: Vec2<f64>, root_position: Vec2<f64>, pressure: f64, tilt: Vec2<f64>, physical_position: Vec2<f64>, button: TabletStylusButton, },
    TabletStylusMotion            { tablet: TabletId, timestamp: Timestamp, window: WindowHandle, position: Vec2<f64>, root_position: Vec2<f64>, pressure: f64, tilt: Vec2<f64>, physical_position: Vec2<f64>, },
    TabletStylusPressed           { tablet: TabletId, timestamp: Timestamp, window: WindowHandle, position: Vec2<f64>, root_position: Vec2<f64>, pressure: f64, tilt: Vec2<f64>, physical_position: Vec2<f64>, },
    TabletStylusRaised            { tablet: TabletId, timestamp: Timestamp, window: WindowHandle, position: Vec2<f64>, root_position: Vec2<f64>, pressure: f64, tilt: Vec2<f64>, physical_position: Vec2<f64>, },
    TabletPadButtonPressedRaw     { tablet: TabletId, timestamp: Timestamp, button: TabletPadButton, },
    TabletPadButtonReleasedRaw    { tablet: TabletId, timestamp: Timestamp, button: TabletPadButton, },
    TabletStylusToolTypeRaw       { tablet: TabletId, timestamp: Timestamp, tool_type: TabletStylusToolType, },
    TabletStylusButtonPressedRaw  { tablet: TabletId, timestamp: Timestamp, pressure: f64, tilt: Vec2<f64>, physical_position: Vec2<f64>, },
    TabletStylusButtonReleasedRaw { tablet: TabletId, timestamp: Timestamp, pressure: f64, tilt: Vec2<f64>, physical_position: Vec2<f64>, },
    TabletStylusMotionRaw         { tablet: TabletId, timestamp: Timestamp, pressure: f64, tilt: Vec2<f64>, physical_position: Vec2<f64>, },
    TabletStylusPressedRaw        { tablet: TabletId, timestamp: Timestamp, pressure: f64, tilt: Vec2<f64>, physical_position: Vec2<f64>, },
    TabletStylusRaisedRaw         { tablet: TabletId, timestamp: Timestamp, pressure: f64, tilt: Vec2<f64>, physical_position: Vec2<f64>, },

    ControllerConnected      { controller: ControllerId, timestamp: Timestamp, },
    ControllerDisconnected   { controller: ControllerId, timestamp: Timestamp, },
    ControllerButtonPressed  { controller: ControllerId, timestamp: Timestamp, button: ControllerButton, },
    ControllerButtonReleased { controller: ControllerId, timestamp: Timestamp, button: ControllerButton, },
    ControllerAxisMotion     { controller: ControllerId, timestamp: Timestamp, axis: ControllerAxis, value: f64, },
}

