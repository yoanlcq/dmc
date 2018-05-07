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

/// An event, as reported by the platform.
///
/// On some targets, there is no actual information about (or concept of)
/// mutiple mice and keyboards; In these cases, an ID is still yielded, but
/// this crate treats it as a spurious one and associates it to an imaginary device
/// that resolves to whatever can be meaningfully obtained from the platform.
///
/// Note that it makes sense to receive both mouse events and tablet events when moving
/// the stylus on a tablet; The device is indeed actually both a mouse and a tablet.
///
/// This crate does little to no post-processing, in order to reduce the maintenance burden.
/// You are encouraged to write your own facilities for this if necessary.
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

    HidConnected { hid: HidID, timestamp: Timestamp, },
    HidDisconnected { hid: HidID, timestamp: Timestamp, },

    // User note: in MouseScroll, the y value is positive when "scrolling up"
    // (that is, pushing the wheel forwards) and negative otherwise.
    MouseEnter             { mouse: HidID, window: WindowHandle, timestamp: Timestamp, position: Vec2<f64>, root_position: Vec2<f64>, is_grabbed: bool,  is_focused: bool, },
    MouseLeave             { mouse: HidID, window: WindowHandle, timestamp: Timestamp, position: Vec2<f64>, root_position: Vec2<f64>, was_grabbed: bool, was_focused: bool, },
    MouseButtonPressed     { mouse: HidID, window: WindowHandle, timestamp: Timestamp, position: Vec2<f64>, root_position: Vec2<f64>, button: MouseButton, clicks: Option<u32>, },
    MouseButtonReleased    { mouse: HidID, window: WindowHandle, timestamp: Timestamp, position: Vec2<f64>, root_position: Vec2<f64>, button: MouseButton, },
    MouseScroll            { mouse: HidID, window: WindowHandle, timestamp: Timestamp, position: Vec2<f64>, root_position: Vec2<f64>, scroll: Vec2<i32>, },
    MouseMotion            { mouse: HidID, window: WindowHandle, timestamp: Timestamp, position: Vec2<f64>, root_position: Vec2<f64>, },
    MouseButtonPressedRaw  { mouse: HidID, timestamp: Timestamp, button: MouseButton, },
    MouseButtonReleasedRaw { mouse: HidID, timestamp: Timestamp, button: MouseButton, },
    MouseScrollRaw         { mouse: HidID, timestamp: Timestamp, scroll: Vec2<i32>, },
    MouseMotionRaw         { mouse: HidID, timestamp: Timestamp, displacement: Vec2<f64>, },

    // Keyboard
    KeyboardFocusGained    { keyboard: HidID, window: WindowHandle, },
    KeyboardFocusLost      { keyboard: HidID, window: WindowHandle, },
    KeyboardKeyPressed     { keyboard: HidID, window: WindowHandle, timestamp: Timestamp, key: Key, is_repeat: bool, text: Option<String>, },
    KeyboardKeyReleased    { keyboard: HidID, window: WindowHandle, timestamp: Timestamp, key: Key, },
    KeyboardKeyPressedRaw  { keyboard: HidID, timestamp: Timestamp, key: Key, },
    KeyboardKeyReleasedRaw { keyboard: HidID, timestamp: Timestamp, key: Key, },

    // Touch (Touchpad, Touch-screen, ....)
    TouchFingerPressed  { touch: HidID, timestamp: Timestamp, finger: u32, pressure: f64, normalized_position: Vec2<f64>, },
    TouchFingerReleased { touch: HidID, timestamp: Timestamp, finger: u32, pressure: f64, normalized_position: Vec2<f64>, },
    TouchFingerMotion   { touch: HidID, timestamp: Timestamp, finger: u32, pressure: f64, normalized_motion:   Vec2<f64>, },
    TouchMultiGesture   { touch: HidID, timestamp: Timestamp, nb_fingers: usize, rotation_radians: f64, pinch: f64, normalized_center: Vec2<f64>, },
    // NOTE: Missing raw events

    TabletPadButtonPressed        { tablet: HidID, timestamp: Timestamp, window: WindowHandle, button: TabletPadButton, },
    TabletPadButtonReleased       { tablet: HidID, timestamp: Timestamp, window: WindowHandle, button: TabletPadButton, },
    TabletStylusToolType          { tablet: HidID, timestamp: Timestamp, window: WindowHandle, position: Vec2<f64>, root_position: Vec2<f64>, tool_type: TabletStylusToolType, },
    TabletStylusButtonPressed     { tablet: HidID, timestamp: Timestamp, window: WindowHandle, position: Vec2<f64>, root_position: Vec2<f64>, pressure: f64, tilt: Vec2<f64>, physical_position: Vec2<f64>, button: TabletStylusButton, },
    TabletStylusButtonReleased    { tablet: HidID, timestamp: Timestamp, window: WindowHandle, position: Vec2<f64>, root_position: Vec2<f64>, pressure: f64, tilt: Vec2<f64>, physical_position: Vec2<f64>, button: TabletStylusButton, },
    TabletStylusMotion            { tablet: HidID, timestamp: Timestamp, window: WindowHandle, position: Vec2<f64>, root_position: Vec2<f64>, pressure: f64, tilt: Vec2<f64>, physical_position: Vec2<f64>, },
    TabletStylusPressed           { tablet: HidID, timestamp: Timestamp, window: WindowHandle, position: Vec2<f64>, root_position: Vec2<f64>, pressure: f64, tilt: Vec2<f64>, physical_position: Vec2<f64>, },
    TabletStylusRaised            { tablet: HidID, timestamp: Timestamp, window: WindowHandle, position: Vec2<f64>, root_position: Vec2<f64>, pressure: f64, tilt: Vec2<f64>, physical_position: Vec2<f64>, },
    TabletPadButtonPressedRaw     { tablet: HidID, timestamp: Timestamp, button: TabletPadButton, },
    TabletPadButtonReleasedRaw    { tablet: HidID, timestamp: Timestamp, button: TabletPadButton, },
    TabletStylusToolTypeRaw       { tablet: HidID, timestamp: Timestamp, tool_type: TabletStylusToolType, },
    TabletStylusButtonPressedRaw  { tablet: HidID, timestamp: Timestamp, pressure: f64, tilt: Vec2<f64>, physical_position: Vec2<f64>, },
    TabletStylusButtonReleasedRaw { tablet: HidID, timestamp: Timestamp, pressure: f64, tilt: Vec2<f64>, physical_position: Vec2<f64>, },
    TabletStylusMotionRaw         { tablet: HidID, timestamp: Timestamp, pressure: f64, tilt: Vec2<f64>, physical_position: Vec2<f64>, },
    TabletStylusPressedRaw        { tablet: HidID, timestamp: Timestamp, pressure: f64, tilt: Vec2<f64>, physical_position: Vec2<f64>, },
    TabletStylusRaisedRaw         { tablet: HidID, timestamp: Timestamp, pressure: f64, tilt: Vec2<f64>, physical_position: Vec2<f64>, },

    ControllerButtonPressed  { controller: HidID, timestamp: Timestamp, button: ControllerButton, },
    ControllerButtonReleased { controller: HidID, timestamp: Timestamp, button: ControllerButton, },
    ControllerAxisMotion     { controller: HidID, timestamp: Timestamp, axis: ControllerAxis, value: f64, },
    // NOTE: value (f64) above is not a normalized value. It is the raw value cast to an f64. The
    // user has to look up the axis info to know the (min,max) and deal with it.
    // The reason is, there are too many situations to handle:
    // - min < 0 && 0 > max
    // - min > 0 && 0 < max
    // - min < 0 && 0 < max
    // - min > 0 && 0 > max (we never know! there might be buggy drivers).
}


impl Event {
    /// Gets the timestamp for this event, if any. This is useful for sorting events (this crate
    /// actually does this automatically when it thinks it makes sense to do so, but do not rely on this!).
    pub fn timestamp(&self) -> Option<Timestamp> {
        match *self {
            Event::Quit => None,
            Event::AppBeingTerminatedByOS => None,
            Event::AppLowMemory => None,
            Event::AppWillEnterBackground => None,
            Event::AppEnteredBackground => None,
            Event::AppWillEnterForeground => None,
            Event::AppEnteredForeground => None,
            Event::SessionEndRequested => None,
            Event::SessionEnding => None,
            Event::WindowShown          { window: _, } => None,
            Event::WindowHidden         { window: _, } => None,
            Event::WindowNeedsRedraw    { window: _, zone: _, more_to_follow: _, } => None,
            Event::WindowMoved          { window: _, position: _, by_user: _, } => None,
            Event::WindowResized        { window: _, size: _, by_user: _, } => None,
            Event::WindowMinimized      { window: _, } => None,
            Event::WindowMaximized      { window: _, } => None,
            Event::WindowUnminized      { window: _, } => None,
            Event::WindowCloseRequested { window: _, } => None,
            Event::HidConnected    { hid: _, timestamp, } => Some(timestamp),
            Event::HidDisconnected { hid: _, timestamp, } => Some(timestamp),
            Event::MouseEnter             { mouse: _, timestamp, window: _, position: _, root_position: _, is_grabbed: _,  is_focused: _, } => Some(timestamp),
            Event::MouseLeave             { mouse: _, timestamp, window: _, position: _, root_position: _, was_grabbed: _, was_focused: _, } => Some(timestamp),
            Event::MouseButtonPressed     { mouse: _, timestamp, window: _, position: _, root_position: _, button: _, clicks: _, } => Some(timestamp),
            Event::MouseButtonReleased    { mouse: _, timestamp, window: _, position: _, root_position: _, button: _, } => Some(timestamp),
            Event::MouseScroll            { mouse: _, timestamp, window: _, position: _, root_position: _, scroll: _, } => Some(timestamp),
            Event::MouseMotion            { mouse: _, timestamp, window: _, position: _, root_position: _, } => Some(timestamp),
            Event::MouseButtonPressedRaw  { mouse: _, timestamp, button: _, } => Some(timestamp),
            Event::MouseButtonReleasedRaw { mouse: _, timestamp, button: _, } => Some(timestamp),
            Event::MouseScrollRaw         { mouse: _, timestamp, scroll: _, } => Some(timestamp),
            Event::MouseMotionRaw         { mouse: _, timestamp, displacement: _, } => Some(timestamp),
            Event::KeyboardFocusGained    { keyboard: _, window: _, } => None,
            Event::KeyboardFocusLost      { keyboard: _, window: _, } => None,
            Event::KeyboardKeyPressed     { keyboard: _, window: _, timestamp, key: _, is_repeat: _, text: _, } => Some(timestamp),
            Event::KeyboardKeyReleased    { keyboard: _, window: _, timestamp, key: _, } => Some(timestamp),
            Event::KeyboardKeyPressedRaw  { keyboard: _, timestamp, key: _, } => Some(timestamp),
            Event::KeyboardKeyReleasedRaw { keyboard: _, timestamp, key: _, } => Some(timestamp),
            Event::TouchFingerPressed  { touch: _, timestamp, finger: _, pressure: _, normalized_position: _, } => Some(timestamp),
            Event::TouchFingerReleased { touch: _, timestamp, finger: _, pressure: _, normalized_position: _, } => Some(timestamp),
            Event::TouchFingerMotion   { touch: _, timestamp, finger: _, pressure: _, normalized_motion:   _, } => Some(timestamp),
            Event::TouchMultiGesture   { touch: _, timestamp, nb_fingers: _, rotation_radians: _, pinch: _, normalized_center: _, } => Some(timestamp),
            Event::TabletPadButtonPressed        { tablet: _, timestamp, window: _, button: _, } => Some(timestamp),
            Event::TabletPadButtonReleased       { tablet: _, timestamp, window: _, button: _, } => Some(timestamp),
            Event::TabletStylusToolType          { tablet: _, timestamp, window: _, position: _, root_position: _, tool_type: _, } => Some(timestamp),
            Event::TabletStylusButtonPressed     { tablet: _, timestamp, window: _, position: _, root_position: _, pressure: _, tilt: _, physical_position: _, button: _, } => Some(timestamp),
            Event::TabletStylusButtonReleased    { tablet: _, timestamp, window: _, position: _, root_position: _, pressure: _, tilt: _, physical_position: _, button: _, } => Some(timestamp),
            Event::TabletStylusMotion            { tablet: _, timestamp, window: _, position: _, root_position: _, pressure: _, tilt: _, physical_position: _, } => Some(timestamp),
            Event::TabletStylusPressed           { tablet: _, timestamp, window: _, position: _, root_position: _, pressure: _, tilt: _, physical_position: _, } => Some(timestamp),
            Event::TabletStylusRaised            { tablet: _, timestamp, window: _, position: _, root_position: _, pressure: _, tilt: _, physical_position: _, } => Some(timestamp),
            Event::TabletStylusButtonPressedRaw  { tablet: _, timestamp, pressure: _, tilt: _, physical_position: _, } => Some(timestamp),
            Event::TabletStylusButtonReleasedRaw { tablet: _, timestamp, pressure: _, tilt: _, physical_position: _, } => Some(timestamp),
            Event::TabletStylusMotionRaw         { tablet: _, timestamp, pressure: _, tilt: _, physical_position: _, } => Some(timestamp),
            Event::TabletStylusPressedRaw        { tablet: _, timestamp, pressure: _, tilt: _, physical_position: _, } => Some(timestamp),
            Event::TabletStylusRaisedRaw         { tablet: _, timestamp, pressure: _, tilt: _, physical_position: _, } => Some(timestamp),
            Event::TabletPadButtonPressedRaw     { tablet: _, timestamp, button: _, } => Some(timestamp),
            Event::TabletPadButtonReleasedRaw    { tablet: _, timestamp, button: _, } => Some(timestamp),
            Event::TabletStylusToolTypeRaw       { tablet: _, timestamp, tool_type: _, } => Some(timestamp),
            Event::ControllerButtonPressed  { controller: _, timestamp, button: _, } => Some(timestamp),
            Event::ControllerButtonReleased { controller: _, timestamp, button: _, } => Some(timestamp),
            Event::ControllerAxisMotion     { controller: _, timestamp, axis: _, value: _, } => Some(timestamp),
        }
    }
}
