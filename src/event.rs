use std::rc::Rc;
use super::{Vec2, Extent2};
use timeout::Timeout;
use hid;
use hid::*;
use context::Context;
use window::Window;

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum Click {
    Single,
    Double,
}

macro_rules! def_events {
    ($($Event:ident $({ $($field:ident: $Field:ty,)+ })*,)+) => {
        #[derive(Debug, Clone)]
        pub enum Event {
            // Removed the timestamp field because not every X11 has it.
            // Timestamps are mainly useful for (and provided with)
            // key/button/motion events.
            $($Event { /*timestamp: u64,*/ $($($field: $Field,)+)* }),+
        }
    }
}

def_events!{
    // 
    // System-ish events
    //
    Quit,
    AppTerminating,
    AppLowMemory,
    AppEnteringBackground,
    AppEnteredBackground,
    AppEnteringForeground,
    AppEnteredForeground,

    // Asks "Is it OK to terminate you ?". (WM_QUERYENDSESSION)
    SessionEndRequested,
    // Perform clean-up operations here. (WM_ENDSESSION)
    SessionEnding,

    RenderTargetReset,
    DisplayLost,

    // The text input buffer was updated ! Use get_text_input_buffer().
    TextInput,

    // Drag'n drop
    DragAndDropBegin,
    DragAndDropCancel,
    DragAndDropFile { file_path: String, },
    DragAndDropText { text: String, },
    DragAndDropRawData { text: Vec<u8>, },

    // Clipboard ???
    ClipboardChanged,

    // 
    // Window events
    //
    WindowShown          { window: Rc<Window>, },
    WindowHidden         { window: Rc<Window>, },
    WindowContentDamaged { window: Rc<Window>, },
    WindowMovedResized   { window: Rc<Window>, position: Vec2<i32>, size: Extent2<u32>, /* by_user: bool, */ },
    WindowMinimized      { window: Rc<Window>, },
    WindowMaximized      { window: Rc<Window>, },
    WindowRestored       { window: Rc<Window>, },
    WindowCloseRequested { window: Rc<Window>, },

    //
    // HIDs
    //

    // Audio
    AudioOutputDeviceConnected,
    AudioOutputDeviceDisconnected,
    AudioCaptureDeviceConnected,
    AudioCaptureDeviceDisconnected,

    // Mouse
    // if window.is_some() {
    //     position is local to window;
    //     global position is in screen coords;
    // } else {
    //     assert_eq!(position, abs_position);
    // }
    // User note: in MouseScroll, the y value is positive when "scrolling up"
    // (that is, pushing the wheel forwards) and negative otherwise.
    MouseConnected      { mouse: Rc<Mouse>, },
    MouseDisconnected   { mouse: Rc<Mouse>, },
    MouseButtonPressed  { mouse: Rc<Mouse>, window: Option<Rc<Window>>, position: Vec2<u32>, abs_position: Vec2<i32>, displacement: Vec2<i32>, button: MouseButton, click: Click, },
    MouseButtonReleased { mouse: Rc<Mouse>, window: Option<Rc<Window>>, position: Vec2<u32>, abs_position: Vec2<i32>, displacement: Vec2<i32>, button: MouseButton, },
    MouseScroll         { mouse: Rc<Mouse>, window: Option<Rc<Window>>, position: Vec2<u32>, abs_position: Vec2<i32>, displacement: Vec2<i32>, scroll: Vec2<i32>, },
    MouseMotion         { mouse: Rc<Mouse>, window: Option<Rc<Window>>, position: Vec2<u32>, abs_position: Vec2<i32>, displacement: Vec2<i32>, },
    MouseEnter          { mouse: Rc<Mouse>, window:        Rc<Window> , position: Vec2<u32>, abs_position: Vec2<i32>,                          },
    MouseLeave          { mouse: Rc<Mouse>, window:        Rc<Window> , position: Vec2<u32>, abs_position: Vec2<i32>, displacement: Vec2<i32>, },
    MouseFocusGained    { mouse: Rc<Mouse>, window:        Rc<Window> , position: Vec2<u32>, abs_position: Vec2<i32>,                          },
    MouseFocusLost      { mouse: Rc<Mouse>, window:        Rc<Window> , position: Vec2<u32>, abs_position: Vec2<i32>, displacement: Vec2<i32>, },
    // TODO: Trackball features for the mouse.

    // Keyboard
    KeyboardConnected     { keyboard: Rc<Keyboard>, },
    KeyboardDisconnected  { keyboard: Rc<Keyboard>, },
    KeyboardKeyPressed    { keyboard: Rc<Keyboard>, window: Option<Rc<Window>>, vkey: VKey, key: Key, is_repeat: bool, },
    KeyboardKeyReleased   { keyboard: Rc<Keyboard>, window: Option<Rc<Window>>, vkey: VKey, key: Key, },
    KeyboardFocusGained   { keyboard: Rc<Keyboard>, window: Rc<Window>, },
    KeyboardFocusLost     { keyboard: Rc<Keyboard>, window: Rc<Window>, },

    // Touch
    TouchConnected    { touch: Rc<Touch>, },
    TouchDisconnected { touch: Rc<Touch>, },
    FingerPressed     { touch: Rc<Touch>, finger: u32, pressure: Axis1DState, normalized_position: Axis2DState, },
    FingerReleased    { touch: Rc<Touch>, finger: u32, pressure: Axis1DState, normalized_position: Axis2DState, },
    FingerMotion      { touch: Rc<Touch>, finger: u32, pressure: Axis1DState, normalized_motion:   Axis2DState, },
    MultiGesture      { touch: Rc<Touch>, rotation: Axis1DState, pinch: Axis1DState, normalized_center: Axis2DState, finger_count: u32, },

    // Graphics Tablet
    // + TODO Touch features ?
    // + Q: Can we recognize pad buttons ? A: No and actually we don't care that much ??
    // + Q: Can we recognize styli ? A: Yes, WinTab says that styli can be assigned a unique ID (introduced with Intuos tablets).
    // + Q: Can we get the tablet's layout ? (answer: yes, use libwacom)
    // For future extensions, see http://www.wacomeng.com/windows/docs/NotesForTabletAwarePCDevelopers.html
    // FIXME pad_buttons
    PenTabletConnected            { pen_tablet: Rc<PenTablet>, },
    PenTabletDisconnected         { pen_tablet: Rc<PenTablet>, },
    PenTabletPadButtonPressed     { pen_tablet: Rc<PenTablet>, window: Option<Rc<Window>>, button: u32, },
    PenTabletPadButtonReleased    { pen_tablet: Rc<PenTablet>, window: Option<Rc<Window>>, button: u32, },
    PenTabletStylusToolType       { pen_tablet: Rc<PenTablet>, window: Option<Rc<Window>>, position: Vec2<u32>, abs_position: Vec2<i32>, tool_type: ToolType, },
    PenTabletStylusButtonPressed  { pen_tablet: Rc<PenTablet>, window: Option<Rc<Window>>, position: Vec2<u32>, abs_position: Vec2<i32>, pressure: Axis1DState, tilt: Axis2DState, raw_position: Axis2DState, },
    PenTabletStylusButtonReleased { pen_tablet: Rc<PenTablet>, window: Option<Rc<Window>>, position: Vec2<u32>, abs_position: Vec2<i32>, pressure: Axis1DState, tilt: Axis2DState, raw_position: Axis2DState, },
    PenTabletStylusMotion         { pen_tablet: Rc<PenTablet>, window: Option<Rc<Window>>, position: Vec2<u32>, abs_position: Vec2<i32>, pressure: Axis1DState, tilt: Axis2DState, raw_position: Axis2DState, },
    PenTabletStylusPressed        { pen_tablet: Rc<PenTablet>, window: Option<Rc<Window>>, position: Vec2<u32>, abs_position: Vec2<i32>, pressure: Axis1DState, tilt: Axis2DState, raw_position: Axis2DState, },
    PenTabletStylusReleased       { pen_tablet: Rc<PenTablet>, window: Option<Rc<Window>>, position: Vec2<u32>, abs_position: Vec2<i32>, pressure: Axis1DState, tilt: Axis2DState, raw_position: Axis2DState, },

    // TODO: Haptic features for those three.
    // Gamepad
    // Joystick
    // Steering Wheel
    // rationale : udev treats all these as ID_INPUT_JOYSTICK.
    ControllerConnected      { controller: Rc<Controller>, },
    ControllerDisconnected   { controller: Rc<Controller>, },
    ControllerButtonPressed  { controller: Rc<Controller>, button: ControllerButton, },
    ControllerButtonReleased { controller: Rc<Controller>, button: ControllerButton, },
    Controller3DAxisMotion   { controller: Rc<Controller>, axis: ControllerAxis1D, state: Axis1DState, },
    Controller2DAxisMotion   { controller: Rc<Controller>, axis: ControllerAxis2D, state: Axis2DState, },
    Controller1DAxisMotion   { controller: Rc<Controller>, axis: ControllerAxis3D, state: Axis3DState, },
    // Axis current value is separated. Axis contains:
    // - Copy of axis info (minmax, deadzone, fuzz, etc)
    // - axis identifier (enum)

}

#[derive(Debug)]
pub struct PollIter<'c> {
    pub(crate) context: &'c mut Context,
}
#[derive(Debug)]
pub struct PeekIter<'c> {
    pub(crate) context: &'c mut Context,
}
#[derive(Debug)]
pub struct WaitIter<'c> {
    pub(crate) context: &'c mut Context,
    pub(crate) timeout: Timeout,
}

impl<'c> Iterator for PollIter<'c> {
    type Item = Event;
    fn next(&mut self) -> Option<Event> {
        self.context.0.poll_next_event()
    }
}

impl<'c> Iterator for PeekIter<'c> {
    type Item = Event;
    fn next(&mut self) -> Option<Event> {
        unimplemented!{}
    }
}

impl<'c> Iterator for WaitIter<'c> {
    type Item = Event;
    fn next(&mut self) -> Option<Event> {
        self.context.0.wait_next_event(self.timeout)
    }
}
