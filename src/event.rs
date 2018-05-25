//! Events reported by the platform.
//!
//! # F.A.Q
//!
//! ## What's an `EventInstant`?
//!
//! A type similar to `std::time::Instant` for events reported by the platform.  
//! Depending on the implementation, it may wrap an `enum` that has as many
//! variants as there are possible timestamp sources.
//!
//! For instance on Linux, `X11` and `evdev` both report timestamps, but they are
//! each relative to their own specific point in time, so it doesn't make sense
//! to compare them, which in turns means we can't reliably sort events based on this.  
//! Therefore, that could call for an `enum` that either contains an
//! `X11` timestamp or an `evdev` timestamp.
//!
//!
//! ## What's the point of the `instant` members in some events?
//!
//! Essentially, the moment your application _handles_ the event is not the same
//! as the moment the event was _sent by the hardware_.
//!
//! You would rather be interested
//! by the latter for computing input sequences (e.g double clicks, the precise time span
//! within which some button was held down, etc).
//!
//!
//! ## Why don't some events have an `instant`?
//!
//! This happens when most of the following conditions are true:
//!
//! - One or more backends don't report it, in which case there's not much we can do;
//! - It is not particularly useful for this kind of event;
//! - It is not particularly _relevant_ to this kind of event.
//!
//!
//! ## Does this crate perform any kind of post-processing on events?
//!
//! The answer is "the least amount possible", for the sake of easing maintenance.  
//! Dealing with platform-specific quirky APIs is enough of a pain already.
//!
//!
//! ## Are events always sorted by instant?
//!
//! They are mostly supposed to! But this crate never guarantees it, for the following reasons :
//!
//! - Backends or drivers do whatever they want. Nothing prevents them from reporting events
//!   in the order they like (even though it's not supposed to happen).
//! - It's not always possible, as `EventInstant` shows. Comparing timestamps just isn't always
//!   possible when there are multiple APIs (i.e timestamp sources) involved, e.g under Linux.
//!
//! However, this crate tries its best to process events in the most sensible way possible.
//! For instance, if you're receiving an event from some gamepad device, it's likely that the next
//! event will come from that same gamepad, with an `instant` you can compare to the previous.
//!
//! In fact...  
//! - You should _probably_ expect `EventInstant`s to be comparable when they come from a same device, with a similar event type.  
//!   Comparing instants of `ControllerAxisMotion` and `ControllerButtonPressed` events is OK if they are from the same device.  
//!   Comparing instants of `DeviceConnected` and `ControllerButtonPressed` events is **probably not** OK, but it may be, on some platforms.
//! - You should _probably_ not expect `EventInstant`s to be comparable when they don't come from a same device.  
//!
//!
//! ## Why are axis values / mouse positions `f64`?
//!
//! See the `device` module's FAQ.

use std::cmp::Ordering;
use std::time::Duration;
use std::ops::{Add, Sub, AddAssign, SubAssign};
use timeout::Timeout;
use super::{Vec2, Extent2, Rect};
use context::Context;
use error;
use window::WindowHandle;
use os::{OsEventInstant, OsUnprocessedEvent};
use device::*;

/// A platform-specific timestamp for an event, starting from an unspecified instant.
///
/// This type exists because, for a given platform, there may be multiple APIs in play,
/// each of which reports timestamps which are relative to specific instants.  
///
/// For instance, on Linux, X11 reports timestamps as a duration since the X server was
/// initialized, but `udev` and `struct input_event`s report timestamps as a duration since
/// whichever relevant kernel service was initialized.  
/// Therefore, computing the duration between an `input_event` timestamp and an X11 timestamp
/// will give wrong results.
/// This type allows computing that duration as long as it makes sense; otherwise the result is
/// just `None`.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, PartialOrd)]
pub struct EventInstant(pub(crate) OsEventInstant);

impl EventInstant {
    /// Returns the amount of time elapsed from another `EventInstant` to this one, if they
    /// originate from the same time source, and this one is not earlier than `earlier`.
    ///
    /// This function does not panic if `earlier` is later than `self` and returns `None` instead.
    pub fn duration_since(&self, earlier: Self) -> Option<Duration> {
        self.0.partial_cmp(&earlier.0).map(|ordering| match ordering {
            Ordering::Less => None,
            _ => self.0.duration_since(earlier.0),
        }).unwrap_or(None)
    }
    /// Returns the absolute difference from another `EventInstant` to this one, as a `Duration`,
    /// if they originate from the same time source.
    pub fn abs_sub(&self, other: Self) -> Option<Duration> {
        self.0.partial_cmp(&other.0).map(|ordering| match ordering {
            Ordering::Less => other.0.duration_since(self.0),
            _ => self.0.duration_since(other.0),
        }).unwrap_or(None)
    }
}

impl Sub<EventInstant> for EventInstant {
    type Output = Option<Duration>;
    fn sub(self, rhs: Self) -> Self::Output {
        self.duration_since(rhs)
    }
}
impl Add<Duration> for EventInstant {
    type Output = Self;
    fn add(self, rhs: Duration) -> Self {
        EventInstant(self.0.add(rhs))
    }
}
impl AddAssign<Duration> for EventInstant {
    fn add_assign(&mut self, rhs: Duration) {
        self.0.add_assign(rhs)
    }
}
impl Sub<Duration> for EventInstant {
    type Output = Self;
    fn sub(self, rhs: Duration) -> Self {
        EventInstant(self.0.sub(rhs))
    }
}
impl SubAssign<Duration> for EventInstant {
    fn sub_assign(&mut self, rhs: Duration) {
        self.0.sub_assign(rhs)
    }
}

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

/// Opaque wrapper around a platform-specific event, providing methods for
/// retrieving platform-specific associated data.
///
/// See the documentation of `Event::UnprocessedEvent`.
#[derive(Debug, Clone, PartialEq)]
pub struct UnprocessedEvent {
    pub(crate) os_event: OsUnprocessedEvent,
    pub(crate) following: usize,
    pub(crate) was_ignored: bool,
}

impl UnprocessedEvent {
    /// Returns the number of "higher-level" `Event`s that were caused by this
    /// platform-specific event; these events will come next.
    ///
    /// Note that it is legal for this to return zero, which doesn't necessarily mean
    /// that the implementation ignored the event; it only means that it didn't cause
    /// other events to be generated and pushed in the queue.  
    pub fn following(&self) -> usize {
        self.following
    }
    /// Was this event ignored by the implementation?  
    /// 
    /// "Ignored" here is understood as "the implementation didn't do anything with it",
    /// which is a bit vague. Sometimes the platform expects some action to be done
    /// immediately, in which case "doing whatever is the minimal/default thing" also counts as "ignoring".
    /// 
    /// This is intended as a soft hint to help you decide whether or not you should handle
    /// this event yourself.
    pub fn was_ignored(&self) -> bool {
        self.was_ignored
    }
}

// TODO Missing event types:
// - Drag'n drop
// - OpenGL context loss
// - Screen plugged/unplugged
// - Audio device plugged/unplugged
// - Trackball features for the mouse.
// - Missing 'instant' field for most events

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
    /// A platform-specific event, which was or was not handled by this crate.
    /// 
    /// When the platform reports an event to a `Context`, the `Context` first pushes
    /// the event's data into its internal queue, as an `UnprocessedEvent`.  
    /// Then, that platform-specific event may cause `N` more "higher-level" `Event`s
    /// to be generated and reported to you (where `N` may be zero or more, and can
    /// be obtained via `UnprocessedEvent::following()`).
    ///
    /// For instance, on X11, when the server reports an `XKeyEvent`, your `Context`
    /// will report the following events, in order:
    ///
    /// 1. `Event::UnprocessedEvent(UnprocessedEvent::from(x_key_event))`;
    /// 2. `Event::MouseMotion { .. }`;
    /// 3. `Event::KeyboardKeyPressed { .. }` (or `Event::KeyboardKeyReleased { .. }`).
    ///
    /// This variant is provided so that your application can implement extended platform-specific functionality.  
    /// If the functionality proves to be useful, consider suggesting to add it to this crate's API! :)
    UnprocessedEvent(UnprocessedEvent),

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

    DeviceConnected { device: DeviceID, instant: EventInstant, info: DeviceInfo },
    DeviceInfoChanged { device: DeviceID, instant: EventInstant, info: DeviceInfo },
    DeviceDisconnected { device: DeviceID, instant: EventInstant, },

    // User note: in MouseScroll, the y value is positive when "scrolling up"
    // (that is, pushing the wheel forwards) and negative otherwise.
    MouseEnter             { mouse: DeviceID, window: WindowHandle, instant: EventInstant, position: Vec2<f64>, root_position: Vec2<f64>, is_grabbed: bool,  is_focused: bool, },
    MouseLeave             { mouse: DeviceID, window: WindowHandle, instant: EventInstant, position: Vec2<f64>, root_position: Vec2<f64>, was_grabbed: bool, was_focused: bool, },
    MouseButtonPressed     { mouse: DeviceID, window: WindowHandle, instant: EventInstant, position: Vec2<f64>, root_position: Vec2<f64>, button: MouseButton, clicks: Option<u32>, },
    MouseButtonReleased    { mouse: DeviceID, window: WindowHandle, instant: EventInstant, position: Vec2<f64>, root_position: Vec2<f64>, button: MouseButton, },
    MouseScroll            { mouse: DeviceID, window: WindowHandle, instant: EventInstant, position: Vec2<f64>, root_position: Vec2<f64>, scroll: Vec2<i32>, },
    MouseMotion            { mouse: DeviceID, window: WindowHandle, instant: EventInstant, position: Vec2<f64>, root_position: Vec2<f64>, },
    MouseButtonPressedRaw  { mouse: DeviceID, instant: EventInstant, button: MouseButton, },
    MouseButtonReleasedRaw { mouse: DeviceID, instant: EventInstant, button: MouseButton, },
    MouseScrollRaw         { mouse: DeviceID, instant: EventInstant, scroll: Vec2<i32>, },
    MouseMotionRaw         { mouse: DeviceID, instant: EventInstant, displacement: Vec2<f64>, },

    // Keyboard
    KeyboardFocusGained    { keyboard: DeviceID, window: WindowHandle, },
    KeyboardFocusLost      { keyboard: DeviceID, window: WindowHandle, },
    KeyboardKeyPressed     { keyboard: DeviceID, window: WindowHandle, instant: EventInstant, key: Key, is_repeat: bool, text: Option<String>, },
    KeyboardKeyReleased    { keyboard: DeviceID, window: WindowHandle, instant: EventInstant, key: Key, },
    KeyboardKeyPressedRaw  { keyboard: DeviceID, instant: EventInstant, key: Key, },
    KeyboardKeyReleasedRaw { keyboard: DeviceID, instant: EventInstant, key: Key, },

    // Touch (Touchpad, Touch-screen, ....)
    TouchFingerPressed  { touch: DeviceID, instant: EventInstant, finger: u32, pressure: f64, normalized_position: Vec2<f64>, },
    TouchFingerReleased { touch: DeviceID, instant: EventInstant, finger: u32, pressure: f64, normalized_position: Vec2<f64>, },
    TouchFingerMotion   { touch: DeviceID, instant: EventInstant, finger: u32, pressure: f64, normalized_motion:   Vec2<f64>, },
    TouchMultiGesture   { touch: DeviceID, instant: EventInstant, nb_fingers: usize, rotation_radians: f64, pinch: f64, normalized_center: Vec2<f64>, },
    // NOTE: Missing raw events

    TabletPadButtonPressed        { tablet: DeviceID, instant: EventInstant, window: WindowHandle, button: TabletPadButton, },
    TabletPadButtonReleased       { tablet: DeviceID, instant: EventInstant, window: WindowHandle, button: TabletPadButton, },
    TabletStylusToolType          { tablet: DeviceID, instant: EventInstant, window: WindowHandle, position: Vec2<f64>, root_position: Vec2<f64>, tool_type: TabletStylusToolType, },
    TabletStylusButtonPressed     { tablet: DeviceID, instant: EventInstant, window: WindowHandle, position: Vec2<f64>, root_position: Vec2<f64>, pressure: f64, tilt: Vec2<f64>, physical_position: Vec2<f64>, button: TabletStylusButton, },
    TabletStylusButtonReleased    { tablet: DeviceID, instant: EventInstant, window: WindowHandle, position: Vec2<f64>, root_position: Vec2<f64>, pressure: f64, tilt: Vec2<f64>, physical_position: Vec2<f64>, button: TabletStylusButton, },
    TabletStylusMotion            { tablet: DeviceID, instant: EventInstant, window: WindowHandle, position: Vec2<f64>, root_position: Vec2<f64>, pressure: f64, tilt: Vec2<f64>, physical_position: Vec2<f64>, },
    TabletStylusPressed           { tablet: DeviceID, instant: EventInstant, window: WindowHandle, position: Vec2<f64>, root_position: Vec2<f64>, pressure: f64, tilt: Vec2<f64>, physical_position: Vec2<f64>, },
    TabletStylusRaised            { tablet: DeviceID, instant: EventInstant, window: WindowHandle, position: Vec2<f64>, root_position: Vec2<f64>, pressure: f64, tilt: Vec2<f64>, physical_position: Vec2<f64>, },
    TabletPadButtonPressedRaw     { tablet: DeviceID, instant: EventInstant, button: TabletPadButton, },
    TabletPadButtonReleasedRaw    { tablet: DeviceID, instant: EventInstant, button: TabletPadButton, },
    TabletStylusToolTypeRaw       { tablet: DeviceID, instant: EventInstant, tool_type: TabletStylusToolType, },
    TabletStylusButtonPressedRaw  { tablet: DeviceID, instant: EventInstant, pressure: f64, tilt: Vec2<f64>, physical_position: Vec2<f64>, },
    TabletStylusButtonReleasedRaw { tablet: DeviceID, instant: EventInstant, pressure: f64, tilt: Vec2<f64>, physical_position: Vec2<f64>, },
    TabletStylusMotionRaw         { tablet: DeviceID, instant: EventInstant, pressure: f64, tilt: Vec2<f64>, physical_position: Vec2<f64>, },
    TabletStylusPressedRaw        { tablet: DeviceID, instant: EventInstant, pressure: f64, tilt: Vec2<f64>, physical_position: Vec2<f64>, },
    TabletStylusRaisedRaw         { tablet: DeviceID, instant: EventInstant, pressure: f64, tilt: Vec2<f64>, physical_position: Vec2<f64>, },

    ControllerButtonPressed  { controller: DeviceID, instant: EventInstant, button: ControllerButton, },
    ControllerButtonReleased { controller: DeviceID, instant: EventInstant, button: ControllerButton, },
    ControllerAxisMotion     { controller: DeviceID, instant: EventInstant, axis: ControllerAxis, value: f64, },
    // NOTE: value (f64) above is not a normalized value. It is the raw value cast to an f64. The
    // user has to look up the axis info to know the (min,max) and deal with it.
    // The reason is, there are too many situations to handle:
    // - min < 0 && 0 > max
    // - min > 0 && 0 < max
    // - min < 0 && 0 < max
    // - min > 0 && 0 > max (we never know! there might be buggy drivers).
}


impl Event {
    /// Gets the `EventInstant` for this event, if any.
    pub fn instant(&self) -> Option<EventInstant> {
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
            Event::DeviceConnected      { device: _, instant, info: _, } => Some(instant),
            Event::DeviceInfoChanged    { device: _, instant, info: _, } => Some(instant),
            Event::DeviceDisconnected   { device: _, instant, } => Some(instant),
            Event::MouseEnter             { mouse: _, instant, window: _, position: _, root_position: _, is_grabbed: _,  is_focused: _, } => Some(instant),
            Event::MouseLeave             { mouse: _, instant, window: _, position: _, root_position: _, was_grabbed: _, was_focused: _, } => Some(instant),
            Event::MouseButtonPressed     { mouse: _, instant, window: _, position: _, root_position: _, button: _, clicks: _, } => Some(instant),
            Event::MouseButtonReleased    { mouse: _, instant, window: _, position: _, root_position: _, button: _, } => Some(instant),
            Event::MouseScroll            { mouse: _, instant, window: _, position: _, root_position: _, scroll: _, } => Some(instant),
            Event::MouseMotion            { mouse: _, instant, window: _, position: _, root_position: _, } => Some(instant),
            Event::MouseButtonPressedRaw  { mouse: _, instant, button: _, } => Some(instant),
            Event::MouseButtonReleasedRaw { mouse: _, instant, button: _, } => Some(instant),
            Event::MouseScrollRaw         { mouse: _, instant, scroll: _, } => Some(instant),
            Event::MouseMotionRaw         { mouse: _, instant, displacement: _, } => Some(instant),
            Event::KeyboardFocusGained    { keyboard: _, window: _, } => None,
            Event::KeyboardFocusLost      { keyboard: _, window: _, } => None,
            Event::KeyboardKeyPressed     { keyboard: _, window: _, instant, key: _, is_repeat: _, text: _, } => Some(instant),
            Event::KeyboardKeyReleased    { keyboard: _, window: _, instant, key: _, } => Some(instant),
            Event::KeyboardKeyPressedRaw  { keyboard: _, instant, key: _, } => Some(instant),
            Event::KeyboardKeyReleasedRaw { keyboard: _, instant, key: _, } => Some(instant),
            Event::TouchFingerPressed  { touch: _, instant, finger: _, pressure: _, normalized_position: _, } => Some(instant),
            Event::TouchFingerReleased { touch: _, instant, finger: _, pressure: _, normalized_position: _, } => Some(instant),
            Event::TouchFingerMotion   { touch: _, instant, finger: _, pressure: _, normalized_motion:   _, } => Some(instant),
            Event::TouchMultiGesture   { touch: _, instant, nb_fingers: _, rotation_radians: _, pinch: _, normalized_center: _, } => Some(instant),
            Event::TabletPadButtonPressed        { tablet: _, instant, window: _, button: _, } => Some(instant),
            Event::TabletPadButtonReleased       { tablet: _, instant, window: _, button: _, } => Some(instant),
            Event::TabletStylusToolType          { tablet: _, instant, window: _, position: _, root_position: _, tool_type: _, } => Some(instant),
            Event::TabletStylusButtonPressed     { tablet: _, instant, window: _, position: _, root_position: _, pressure: _, tilt: _, physical_position: _, button: _, } => Some(instant),
            Event::TabletStylusButtonReleased    { tablet: _, instant, window: _, position: _, root_position: _, pressure: _, tilt: _, physical_position: _, button: _, } => Some(instant),
            Event::TabletStylusMotion            { tablet: _, instant, window: _, position: _, root_position: _, pressure: _, tilt: _, physical_position: _, } => Some(instant),
            Event::TabletStylusPressed           { tablet: _, instant, window: _, position: _, root_position: _, pressure: _, tilt: _, physical_position: _, } => Some(instant),
            Event::TabletStylusRaised            { tablet: _, instant, window: _, position: _, root_position: _, pressure: _, tilt: _, physical_position: _, } => Some(instant),
            Event::TabletStylusButtonPressedRaw  { tablet: _, instant, pressure: _, tilt: _, physical_position: _, } => Some(instant),
            Event::TabletStylusButtonReleasedRaw { tablet: _, instant, pressure: _, tilt: _, physical_position: _, } => Some(instant),
            Event::TabletStylusMotionRaw         { tablet: _, instant, pressure: _, tilt: _, physical_position: _, } => Some(instant),
            Event::TabletStylusPressedRaw        { tablet: _, instant, pressure: _, tilt: _, physical_position: _, } => Some(instant),
            Event::TabletStylusRaisedRaw         { tablet: _, instant, pressure: _, tilt: _, physical_position: _, } => Some(instant),
            Event::TabletPadButtonPressedRaw     { tablet: _, instant, button: _, } => Some(instant),
            Event::TabletPadButtonReleasedRaw    { tablet: _, instant, button: _, } => Some(instant),
            Event::TabletStylusToolTypeRaw       { tablet: _, instant, tool_type: _, } => Some(instant),
            Event::ControllerButtonPressed  { controller: _, instant, button: _, } => Some(instant),
            Event::ControllerButtonReleased { controller: _, instant, button: _, } => Some(instant),
            Event::ControllerAxisMotion     { controller: _, instant, axis: _, value: _, } => Some(instant),
        }
    }
}
