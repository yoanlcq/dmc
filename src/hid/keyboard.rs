//! Keyboards.

use context::Context;
use os::{OsKeyboardState, OsKeysym, OsKeycode};
use super::{HidID, KeyState, Result};

/// There's nothing in here, for now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyboardInfo;

/// Most platforms provide a (supposedly) efficient way to query
/// the whole keyboard's state in a single call.
///
/// Under Windows, it's `GetKeyboardState()`.
/// Under X11, it's `XQueryKeymap()`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyboardState(pub(crate) OsKeyboardState);

impl Context {
    /// Lists currently connected keyboard devices.
    pub fn keyboards(&self) -> Result<Vec<HidID>> {
        self.0.keyboards()
    }
    /// Gets the ID for the main keyboard, if any.
    pub fn main_keyboard(&self) -> Result<HidID> {
        self.0.main_keyboard()
    }
    /// Captures the current state of the keyboard which ID is given.
    pub fn keyboard_state(&self, keyboard: HidID) -> Result<KeyboardState> {
        self.0.keyboard_state(keyboard)
    }
    /// Captures the current state of a keyboard's key (by scan code) which ID is given.
    pub fn keyboard_keycode_state(&self, keyboard: HidID, keycode: Keycode) -> Result<KeyState> {
        self.0.keyboard_keycode_state(keyboard, keycode)
    }
    /// Captures the current state of a keyboard's key (by virtual code) which ID is given.
    pub fn keyboard_keysym_state(&self, keyboard: HidID, keysym: Keysym) -> Result<KeyState> {
        self.0.keyboard_keysym_state(keyboard, keysym)
    }
    /// Gets the friendly name for the given key.
    pub fn keysym_name(&self, keysym: Keysym) -> Result<String> {
        self.0.keysym_name(keysym)
    }
    /// Translates a scan code to a key code for the keyboard which ID is given.
    pub fn keysym_from_keycode(&self, keyboard: HidID, keycode: Keycode) -> Result<Keysym> {
        self.0.keysym_from_keycode(keyboard, keycode)
    }
    /// Retrieves the scan code that would translate to the given key code for the keyboard which ID is given.
    pub fn keycode_from_keysym(&self, keyboard: HidID, keysym: Keysym) -> Result<Keycode> {
        self.0.keycode_from_keysym(keyboard, keysym)
    }
}

impl KeyboardState {
    /// Gets the state of the given key, by scan code.
    pub fn keycode(&self, keycode: Keycode) -> Option<KeyState> {
        self.0.keycode(keycode)
    }
    /// Gets the state of the given key, by virtual key code.
    pub fn keysym(&self, keysym: Keysym) -> Option<KeyState> {
        self.0.keysym(keysym)
    }
}

/// A hardware-given integer that uniquely identifies a key location for a specific keyboard.
///
/// This value normally fits into an unsigned 8-bit integer and ranges from 8 to 255.  
/// It is often named "scan code", but X11 calls it "keycode".
///
/// A scan code is supposedly meaningless without context. To give it meaning,
/// one has to use the operating system's facilities to translate a scan code
/// to a virtual key code (or `Keysym`) with regards to the keyboard's actual layout.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Keycode(pub(crate) OsKeycode);

/// A convenience container for both a `Keycode` and `Keysym`.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct Key {
    /// The scan code.
    pub code: Keycode,
    /// The virtual code; May be `None` to indicate that the lookup failed for some reason,
    /// but this should rarely occur.
    pub sym: Option<Keysym>,
}

/// A virtual key code, i.e the OS-provided specific meaning of a key for a keyboard.
///
/// Windows calls it "VKey", X11 calls it "Keysym".
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum Keysym {
    Other(OsKeysym),
    Num1,
    Num2,
    Num3,
    Num4,
    Num5,
    Num6,
    Num7,
    Num8,
    Num9,
    Num0,
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    F21,
    F22,
    F23,
    F24,

    Esc,
    Space,
    Backspace,
    Tab,
    Enter,

    CapsLock,
    NumLock,
    ScrollLock,

    Minus,
    Equal,
    LeftBrace,
    RightBrace,
    Semicolon,
    Apostrophe,
    Grave,
    Comma,
    Dot,
    Slash,
    Backslash,

    LCtrl,
    RCtrl,
    LShift,
    RShift,
    LAlt,
    RAlt,
    LSystem,
    RSystem,
    LMeta,
    RMeta,
    Compose,

    Home,
    End,

    Up,
    Down,
    Left,
    Right,

    PageUp,
    PageDown,

    Insert,
    Delete,

    SysRQ,
    LineFeed,

    Kp0,
    Kp1,
    Kp2,
    Kp3,
    Kp4,
    Kp5,
    Kp6,
    Kp7,
    Kp8,
    Kp9,
    KpPlus,
    KpMinus,
    KpAsterisk,
    KpSlash,
    KpDot,
    KpEnter,
    KpEqual,
    KpComma,

    Mute,
    VolumeDown,
    VolumeUp,
    NextTrack,
    PrevTrack,
    PlayPause,
    Stop,

    BrowserBack,
    BrowserForward,
    BrowserRefresh,
    BrowserStop,
    BrowserSearch,
    BrowserFavorites,
    BrowserHome,

    LaunchMail,
    LaunchMediaSelect,
    LaunchApp1,
    LaunchApp2,

    Power,
    Sleep,
    Menu,
    Pause,
    Snapshot,
    Select,
    Print,
    Execute,
    Help,
    Apps,

    OemPlus,
    OemComma,
    OemMinus,
    OemPeriod,

    ZenkakuHankaku,
    Katakana,
    Hiragana,
    Henkan,
    KatakanaHiragana,
    Muhenkan,

    Hangul,
    Hanja,
    Yen,

    Junja,
    Final,
    Kanji,
}

