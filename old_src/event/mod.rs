//! Platform-specific event handling.

// INFO:
// --- Detecting graphics tablets ---
// 1. Detect XInput devices
//    $ xinput
// 2. Find the relevant device ID. Let's assume it is 16.
//    List the device's properties (including vendor-specific ones) :
//    $ xinput list-props 16
// 3. This also gives you an event device path.
//    Let's assume it is `/dev/input/event13`.
//    You can get what `udev` has to say about it:
//    $ udevadm info /dev/input/event13
//    If you see `ID_INPUT_TABLET=1`, then it's a tablet!
// 4. See what XInput2 events look like on this device:
//    $ xinput test-xi2 16
//
// References:
// - `libinput` (we can't use it: it is for use by the X server or Wayland compositor).
//   Has nice documentation on graphics tablets.
// - `QTabletEvent`, Qt 5.
//   Illustrates well what we could expect from an API (including physical position
//   events for subpixel precision, etc).
// - The DIGImend project (https://digimend.github.io/). They maintain a big list
//   of less common graphics tablets.
// - `libwacom`, which provides static descriptions for graphics tablets
//   (and not only Wacom tablets actually). It is packaged with a bunch of
//   text files in key-value format (the "database"), and the API is only
//   an abstraction over the retrieval and parsing of some of those files.
//   You can get info about a tablet by getting a handle to it via either:
//   - An event device path (e.g /dev/input/eventXX);
//   - From vendor and product IDs;
//   - From a name.
//   Quoting their doc:
//       libwacom is a library to identify wacom tablets and their model-specific
//       features. It provides easy access to information such as "is this a
//       built-in on-screen tablet", "what is the size of this model", etc.
// - Who-T's blog - "X Input hacker" : http://who-t.blogspot.fr/
// - The Linux Wacom project
//
// TODO:
// - GetNumTouchDevices, etc
// - GetNumTouchFingers, etc
// - LoadDollarTemplates, SaveDollarTemplate, etc
// - RecordGesture
// - Start/StopTextInput
//
// NOTE: Model:
// - One virtual message queue associated to the Display;
// - One virtual message queue associated to each Window;
// The user MUST:
// - First, for each Window, poll Window events:
//   - GetMessage(.. hwnd ...);
//   - XWindowEvent(...);
// - Then poll Display events:
//   - GetMessage(.. NULL ...)
//   - XNextEvent();
//
// OR:
// - XCheckTypedEvent, XMaskEvent and friends;
// - GetMessage(.., -1, ...) ???
//
// !!! IMPORTANT TODO:
// In X11, reply explicitly to _NET_WM_PING messages.
//
//
// TODO be able to provide WM_QUERYENDSESSION and WM_ENDSESSION events (Win32)- See [Shutting Down](https://msdn.microsoft.com/en-us/library/windows/desktop/aa376881(v=vs.85).aspx)

use vek::Extent2;
use vek::Vec2;
use Timeout;


#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub struct VKey;
#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub struct Key;

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum Click {
    Single,
    Double,
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
    Extra1,
    Extra2,
    Extra3,
    Other(u32),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    AudioOutputDeviceAdded,
    AudioOutputDeviceRemoved,
    AudioCaptureDeviceAdded,
    AudioCaptureDeviceRemoved,

    HidConnected { hid: HidId, },
    HidDisconnected { hid: HidId, },
    HidRemapped { hid: HidId, }, // Just says that the controller mappings have changed and might need to be refreshed.
    HidButtonPressed { hid: HidId, button: u32 },
    HidButtonReleased { hid: HidId, button: u32 },
    HidAxisMotion { hid: HidId, axis_id: u32, axis: Vec2<i32> },
    HidTrackballMotion { hid: HidId, ball_index: u8, motion: Vec2<i32> },

    DollarGesture { touch_device_id: u32, gesture_id: u32, finger_count: u8, error: f32, normalized_center: Vec2<f32> },

    DragAndDropBegin,
    DragAndDropCancel,
    DragAndDropFile { file_path: String, },
    DragAndDropText { text: String, },
    DragAndDropRawData { text: Vec<u8>, },

    FingerPressed { touch_id: u32, finger_id: u32, normalized_position: Vec2<f32>, pressure: f32 },
    FingerReleased { touch_id: u32, finger_id: u32, normalized_position: Vec2<f32>, pressure: f32 },
    FingerMotion { touch_id: u32, finger_id: u32, normalized_motion: Vec2<f32>, pressure: f32 },

    MultiGesture { touch_id: u32, theta: f32, dist: f32, normalized_center: Vec2<f32>, finger_count: u8 },

    KeyPressed { window_id: Option<WindowId>, is_repeat: bool, vkey: VKey, key: Key, },
    KeyReleased { window_id: Option<WindowId>, is_repeat: bool, vkey: VKey, key: Key, },

    MouseButtonPressed { window_id: Option<WindowId>, mouse: u32, click: Click, button: MouseButton, },
    MouseButtonReleased { window_id: Option<WindowId>, mouse: u32, click: Click, button: MouseButton, },
    MouseMotion { window_id: Option<WindowId>, mouse: u32, new_position: Vec2<i32> },
    MouseScroll { window_id: Option<WindowId>, mouse: u32, scroll: Vec2<i32>, },

    WindowShown { window_id: WindowId, },
    WindowHidden { window_id: WindowId, },
    WindowPaint { window_id: WindowId, },
    WindowMoved { window_id: WindowId, position: Extent2<u32>, },
    WindowResized { window_id: WindowId, size: Extent2<u32>, by_user: bool, },
    WindowMinimized { window_id: WindowId, },
    WindowMaximized { window_id: WindowId, },
    WindowRestored { window_id: WindowId, },
    WindowGainedMouseFocus { window_id: WindowId, },
    WindowLostMouseFocus { window_id: WindowId, },
    WindowGainedKeyboardFocus { window_id: WindowId, },
    WindowLostKeyboardFocus { window_id: WindowId, },
    WindowCloseRequested { window_id: WindowId, },

    Quit,
    AppTerminating,
    AppLowMemory,
    AppEnteringBackground,
    AppEnteredBackground,
    AppEnteringForeground,
    AppEnteredForeground,

    /// Asks "Is it OK to terminate you ?". (WM_QUERYENDSESSION)
    SessionEndRequested,
    /// Perform clean-up operations here. (WM_ENDSESSION)
    SessionEnding,

    KeymapChanged,
    ClipboardChanged,
    RenderTargetReset,
    DisplayLost,

    /// The text input buffer was updated ! Use get_text_input_buffer().
    TextInput,
}

#[derive(Debug, Hash)]
pub struct Clipboard {
    raw_data: Vec<u8>,
}
// XXX It's a singleton, how to implement this ?
impl Clipboard {
    pub fn get_raw_buffer<'a>(&'a self) -> &'a [u8] {
        &self.raw_data
    }
    pub fn overwrite_with_utf8(&mut self, _s: &str) {
        unimplemented!()
    }
}
#[derive(Debug, Hash)]
pub struct TextInput {
    raw_data: Vec<u8>,
}

// XXX It's a singleton, how to implement this ?
impl TextInput {
    // XXX is is practical for the user?
    pub fn start(&mut self) -> TextInputRecording { unimplemented!() }
}

pub struct TextInputRecording<'a> {
    _text_input: &'a TextInput,
}
impl<'a> TextInputRecording<'a> {
    pub fn get_raw_buffer(&'a self) -> &'a [u8] {
        unimplemented!()
    }
}


pub mod queue {

    use super::*;

    #[derive(Debug)]
    pub struct EventQueue {}

    #[derive(Debug)]
    pub struct PeekIter<'a> {
        _queue: &'a EventQueue,
    }
    #[derive(Debug)]
    pub struct PollIter<'a> {
        _queue: &'a mut EventQueue,
    }
    #[derive(Debug)]
    pub struct WaitIter<'a> {
        _queue: &'a mut EventQueue,
        _timeout: Timeout,
    }
    impl<'a> Iterator for PollIter<'a> {
        type Item = Event;
        fn next(&mut self) -> Option<Self::Item> {
            unimplemented!()
        }
    }

    impl<'a> Iterator for WaitIter<'a> {
        type Item = Event;
        fn next(&mut self) -> Option<Self::Item> {
            unimplemented!()
        }
    }

    impl<'a> Iterator for PeekIter<'a> {
        type Item = &'a Event;
        fn next(&mut self) -> Option<Self::Item> {
            unimplemented!()
        }
    }

    impl<'a> EventQueue {
        pub fn push(&mut self, _event: Event) { unimplemented!() }
        pub fn poll(&'a mut self) -> PollIter<'a> { 
            PollIter { _queue: self }
        }
        pub fn wait<T: Into<Timeout>>(&'a mut self, timeout: T) -> WaitIter<'a> { 
            WaitIter { _queue: self, _timeout: timeout.into() }
        }
        pub fn peek(&'a self) -> PeekIter<'a> {
            PeekIter { _queue: self }
        }
    }
}

pub use self::queue::EventQueue;

