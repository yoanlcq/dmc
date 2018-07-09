use device::{
    self,
    DeviceID, KeyState,
    KeyboardState, Keysym, Keycode,
};
use os::OsContext;
use super::super::winapi_utils::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OsKeyboardState;
pub type OsKeycode = u8;
pub type OsKeysym = i32;

impl OsContext {
    pub fn main_keyboard(&self) -> device::Result<DeviceID> {
        unimplemented!()
    }
    pub fn keyboard_state(&self, keyboard: DeviceID) -> device::Result<KeyboardState> {
        unimplemented!()
    }
    pub fn keyboard_keycode_state(&self, keyboard: DeviceID, keycode: Keycode) -> device::Result<KeyState> {
        unimplemented!()
    }
    pub fn keyboard_keysym_state(&self, keyboard: DeviceID, keysym: Keysym) -> device::Result<KeyState> {
        unimplemented!()
    }
    pub fn keysym_name(&self, keysym: Keysym) -> device::Result<String> {
        unimplemented!()
    }
    pub fn keysym_from_keycode(&self, keyboard: DeviceID, keycode: Keycode) -> device::Result<Keysym> {
        unimplemented!()
    }
    pub fn keycode_from_keysym(&self, keyboard: DeviceID, keysym: Keysym) -> device::Result<Keycode> {
        unimplemented!()
    }
}

impl OsKeyboardState {
    pub fn keycode(&self, keycode: Keycode) -> Option<KeyState> {
        unimplemented!()
    }
    pub fn keysym(&self, keysym: Keysym) -> Option<KeyState> {
        unimplemented!()
    }
}

macro_rules! vkeys {
    ($($VK:expr => $Keysym:expr,)+) => {
        pub fn keysym_from_vkey(vkey: OsKeysym) -> Keysym {
            match vkey {
                $(vkey if vkey == $VK => $Keysym,)+
                vkey => Keysym::Other(vkey),
            }
        }
        pub fn vkey_from_keysym(keysym: Keysym) -> Option<OsKeysym> {
            match keysym {
                $(keysym if keysym == $Keysym => Some($VK),)+
                _ => None,
            }
        }
    };
}

vkeys!{
    // VK_LBUTTON => Keysym::Other(0x01),
    // VK_RBUTTON => Keysym::Other(0x02),
    // VK_CANCEL => Keysym::Cancel,
    // VK_MBUTTON => Keysym::Other(0x04),
    // VK_XBUTTON1 => Keysym::Other(0x05),
    // VK_XBUTTON2 => Keysym::Other(0x06),
    VK_BACK => Keysym::Backspace,
    VK_TAB => Keysym::Tab,
    // VK_CLEAR => Keysym::Clear,
    VK_RETURN => Keysym::Enter,
    VK_SHIFT => Keysym::LShift,
    VK_CONTROL => Keysym::LCtrl,
    VK_MENU => Keysym::Menu,
    VK_PAUSE => Keysym::Pause,
    VK_CAPITAL => Keysym::CapsLock,
    VK_HANGUL => Keysym::Hangul,
    VK_JUNJA => Keysym::Junja,
    VK_FINAL => Keysym::Final,
    VK_HANJA => Keysym::Hanja,
    VK_KANJI => Keysym::Kanji,
    VK_ESCAPE => Keysym::Esc,
    VK_CONVERT => Keysym::ImeConvert,
    VK_NONCONVERT => Keysym::ImeNonConvert,
    VK_ACCEPT => Keysym::ImeAccept,
    VK_MODECHANGE => Keysym::ImeModeChange,
    VK_SPACE => Keysym::Space,
    VK_PRIOR => Keysym::PageUp,
    VK_NEXT => Keysym::PageDown,
    VK_END => Keysym::End,
    VK_HOME => Keysym::Home,
    VK_LEFT => Keysym::Left,
    VK_UP => Keysym::Up,
    VK_RIGHT => Keysym::Right,
    VK_DOWN => Keysym::Down,
    VK_SELECT => Keysym::Select,
    VK_PRINT => Keysym::Print,
    VK_EXECUTE => Keysym::Execute,
    VK_SNAPSHOT => Keysym::Snapshot,
    VK_INSERT => Keysym::Insert,
    VK_DELETE => Keysym::Delete,
    VK_HELP => Keysym::Help,

    0x30 => Keysym::Num0,
    0x31 => Keysym::Num1,
    0x32 => Keysym::Num2,
    0x33 => Keysym::Num3,
    0x34 => Keysym::Num4,
    0x35 => Keysym::Num5,
    0x36 => Keysym::Num6,
    0x37 => Keysym::Num7,
    0x38 => Keysym::Num8,
    0x39 => Keysym::Num9,

    0x41 => Keysym::A,
    0x42 => Keysym::B,
    0x43 => Keysym::C,
    0x44 => Keysym::D,
    0x45 => Keysym::E,
    0x46 => Keysym::F,
    0x47 => Keysym::G,
    0x48 => Keysym::H,
    0x49 => Keysym::I,
    0x4A => Keysym::J,
    0x4B => Keysym::K,
    0x4C => Keysym::L,
    0x4D => Keysym::M,
    0x4E => Keysym::N,
    0x4F => Keysym::O,
    0x50 => Keysym::P,
    0x51 => Keysym::Q,
    0x52 => Keysym::R,
    0x53 => Keysym::S,
    0x54 => Keysym::T,
    0x55 => Keysym::U,
    0x56 => Keysym::V,
    0x57 => Keysym::W,
    0x58 => Keysym::X,
    0x59 => Keysym::Y,
    0x5A => Keysym::Z,
    
    VK_LWIN => Keysym::LSystem,
    VK_RWIN => Keysym::RSystem,
    VK_APPS => Keysym::Apps,
    VK_SLEEP => Keysym::Sleep,
    VK_NUMPAD0 => Keysym::Kp0,
    VK_NUMPAD1 => Keysym::Kp1,
    VK_NUMPAD2 => Keysym::Kp2,
    VK_NUMPAD3 => Keysym::Kp3,
    VK_NUMPAD4 => Keysym::Kp4,
    VK_NUMPAD5 => Keysym::Kp5,
    VK_NUMPAD6 => Keysym::Kp6,
    VK_NUMPAD7 => Keysym::Kp7,
    VK_NUMPAD8 => Keysym::Kp8,
    VK_NUMPAD9 => Keysym::Kp9,
    VK_MULTIPLY => Keysym::KpAsterisk,
    VK_ADD => Keysym::KpPlus,
    VK_SEPARATOR => Keysym::KpComma,
    VK_SUBTRACT => Keysym::KpMinus,
    VK_DECIMAL => Keysym::KpDot,
    VK_DIVIDE => Keysym::KpSlash,
    VK_F1  => Keysym::F1,
    VK_F2  => Keysym::F2,
    VK_F3  => Keysym::F3,
    VK_F4  => Keysym::F4,
    VK_F5  => Keysym::F5,
    VK_F6  => Keysym::F6,
    VK_F7  => Keysym::F7,
    VK_F8  => Keysym::F8,
    VK_F9  => Keysym::F9,
    VK_F10 => Keysym::F10,
    VK_F11 => Keysym::F11,
    VK_F12 => Keysym::F12,
    VK_F13 => Keysym::F13,
    VK_F14 => Keysym::F14,
    VK_F15 => Keysym::F15,
    VK_F16 => Keysym::F16,
    VK_F17 => Keysym::F17,
    VK_F18 => Keysym::F18,
    VK_F19 => Keysym::F19,
    VK_F20 => Keysym::F20,
    VK_F21 => Keysym::F21,
    VK_F22 => Keysym::F22,
    VK_F23 => Keysym::F23,
    VK_F24 => Keysym::F24,
    VK_NUMLOCK => Keysym::NumLock,
    VK_SCROLL => Keysym::ScrollLock,
    // ---- OEM-specific
    // VK_OEM_NEC_EQUAL => 0x92,
    // VK_OEM_FJ_JISHO => 0x92,
    // VK_OEM_FJ_MASSHOU => 0x93,
    // VK_OEM_FJ_TOUROKU => 0x94,
    // VK_OEM_FJ_LOYA => 0x95,
    // VK_OEM_FJ_ROYA => 0x96,
    VK_LSHIFT => Keysym::LShift,
    VK_RSHIFT => Keysym::RShift,
    VK_LCONTROL => Keysym::LCtrl,
    VK_RCONTROL => Keysym::RCtrl,
    VK_LMENU => Keysym::LMeta,
    VK_RMENU => Keysym::RMeta,
    VK_BROWSER_BACK => Keysym::BrowserBack,
    VK_BROWSER_FORWARD => Keysym::BrowserForward,
    VK_BROWSER_REFRESH => Keysym::BrowserRefresh,
    VK_BROWSER_STOP => Keysym::BrowserStop,
    VK_BROWSER_SEARCH => Keysym::BrowserSearch,
    VK_BROWSER_FAVORITES => Keysym::BrowserFavorites,
    VK_BROWSER_HOME => Keysym::BrowserHome,
    VK_VOLUME_MUTE => Keysym::Mute,
    VK_VOLUME_DOWN => Keysym::VolumeDown,
    VK_VOLUME_UP => Keysym::VolumeUp,
    VK_MEDIA_NEXT_TRACK => Keysym::NextTrack,
    VK_MEDIA_PREV_TRACK => Keysym::PrevTrack,
    VK_MEDIA_STOP => Keysym::Stop,
    VK_MEDIA_PLAY_PAUSE => Keysym::PlayPause,
    VK_LAUNCH_MAIL => Keysym::LaunchMail,
    VK_LAUNCH_MEDIA_SELECT => Keysym::LaunchMediaSelect,
    VK_LAUNCH_APP1 => Keysym::LaunchApp1,
    VK_LAUNCH_APP2 => Keysym::LaunchApp2,
    // VK_OEM_1 => 0xBA,
    VK_OEM_PLUS => Keysym::Plus,
    VK_OEM_COMMA => Keysym::Comma,
    VK_OEM_MINUS => Keysym::Minus,
    VK_OEM_PERIOD => Keysym::Dot,
    // VK_OEM_2 => 0xBF,
    // VK_OEM_3 => 0xC0,
    // VK_OEM_4 => 0xDB,
    // VK_OEM_5 => 0xDC,
    // VK_OEM_6 => 0xDD,
    // VK_OEM_7 => 0xDE,
    // VK_OEM_8 => 0xDF,
    // VK_OEM_AX => 0xE1,
    // VK_OEM_102 => 0xE2,
    // VK_ICO_HELP => 0xE3,
    // VK_ICO_00 => 0xE4,
    // VK_PROCESSKEY => 0xE5,
    // VK_ICO_CLEAR => 0xE6,
    // VK_PACKET => 0xE7,
    // VK_OEM_RESET => 0xE9,
    // VK_OEM_JUMP => 0xEA,
    // VK_OEM_PA1 => 0xEB,
    // VK_OEM_PA2 => 0xEC,
    // VK_OEM_PA3 => 0xED,
    // VK_OEM_WSCTRL => 0xEE,
    // VK_OEM_CUSEL => 0xEF,
    // VK_OEM_ATTN => 0xF0,
    // VK_OEM_FINISH => 0xF1,
    // VK_OEM_COPY => 0xF2,
    // VK_OEM_AUTO => 0xF3,
    // VK_OEM_ENLW => 0xF4,
    // VK_OEM_BACKTAB => 0xF5,
    // VK_ATTN => 0xF6,
    // VK_CRSEL => 0xF7,
    // VK_EXSEL => 0xF8,
    // VK_EREOF => 0xF9,
    // VK_PLAY => 0xFA,
    // VK_ZOOM => 0xFB,
    // VK_NONAME => 0xFC,
    // VK_PA1 => 0xFD,
    // VK_OEM_CLEAR => 0xFE,
}