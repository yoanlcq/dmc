//! Keyboards.

use context::Context;
use os::{OsKeyboardId, OsKeyboardState, OsVKey};
use super::{DeviceId, KeyState, Result};

/// A device ID type for keyboards.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct KeyboardId(pub(crate) OsKeyboardId);
impl DeviceId for KeyboardId {}

/// Opaque wrapper around a platform-specific key code.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct VKey(pub(crate) OsVKey);

/// Most platforms provide a (supposedly) efficient way to query
/// the whole keyboard's state in a single call.
///
/// Under Windows, it's `GetKeyboardState()`.
/// Under X11, it's `XQueryKeymap()`.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct KeyboardState(pub(crate) OsKeyboardState);


impl VKey {
    /// Resolves this virtual key to the matching known key value, if any.
    pub fn translated(self) -> Key {
        unimplemented!{}
    }
}

impl Key {
    /// Gets the `VKey` that yielded this `Key`.
    pub fn untranslated(&self) -> VKey {
        unimplemented!{}
    }
}

impl Context {
    /// Lists currently connected keyboard devices.
    pub fn keyboards(&self) -> Result<Vec<KeyboardId>> {
        unimplemented!{}
    }
    /// Gets the ID for the main keyboard, if any.
    pub fn main_keyboard(&self) -> Result<KeyboardId> {
        unimplemented!{}
    }
    /// Captures the current state of the keyboard which ID is given.
    pub fn keyboard_state(&self, keyboard: KeyboardId) -> Result<KeyboardState> {
        unimplemented!{}
    }
    /// Captures the current state of a keyboard's key which ID is given.
    pub fn keyboard_key_state(&self, keyboard: KeyboardId, key: VKey) -> Result<KeyState> {
        unimplemented!{}
    }
    /// Gets the platform-specific, friendly name for the given VKey.
    pub fn key_name(&self, key: VKey) -> Result<String> {
        unimplemented!{}
    }
}

impl KeyboardState {
    /// Gets the state of the given key.
    pub fn key(&self, key: VKey) -> Option<KeyState> {
        unimplemented!{}
    }
}

macro_rules! keys {
    ($($Key:ident $key:ident,)+) => {
        /// These are NOT appropriate for text input. For this, read the
        /// `text` member of the `VKeyPressed` event instead.
        #[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
        #[allow(missing_docs)]
        pub enum Key {
            $($Key),+,
            Other(VKey),
        }
    };
}

keys!{
    Num1              num1                ,
    Num2              num2                ,
    Num3              num3                ,
    Num4              num4                ,
    Num5              num5                ,
    Num6              num6                ,
    Num7              num7                ,
    Num8              num8                ,
    Num9              num9                ,
    Num0              num0                ,
    A                 a                   ,
    B                 b                   ,
    C                 c                   ,
    D                 d                   ,
    E                 e                   ,
    F                 f                   ,
    G                 g                   ,
    H                 h                   ,
    I                 i                   ,
    J                 j                   ,
    K                 k                   ,
    L                 l                   ,
    M                 m                   ,
    N                 n                   ,
    O                 o                   ,
    P                 p                   ,
    Q                 q                   ,
    R                 r                   ,
    S                 s                   ,
    T                 t                   ,
    U                 u                   ,
    V                 v                   ,
    W                 w                   ,
    X                 x                   ,
    Y                 y                   ,
    Z                 z                   ,
    F1                f1                  ,
    F2                f2                  ,
    F3                f3                  ,
    F4                f4                  ,
    F5                f5                  ,
    F6                f6                  ,
    F7                f7                  ,
    F8                f8                  ,
    F9                f9                  ,
    F10               f10                 ,
    F11               f11                 ,
    F12               f12                 ,
    F13               f13                 ,
    F14               f14                 ,
    F15               f15                 ,
    F16               f16                 ,
    F17               f17                 ,
    F18               f18                 ,
    F19               f19                 ,
    F20               f20                 ,
    F21               f21                 ,
    F22               f22                 ,
    F23               f23                 ,
    F24               f24                 ,

    Esc               esc                 ,
    Space             space               ,
    Backspace         backspace           ,
    Tab               tab                 ,
    Enter             enter               ,

    CapsLock          caps_lock           ,
    NumLock           num_lock            ,
    ScrollLock        scroll_lock         ,

    Minus             minus               ,
    Equal             equal               ,
    LeftBrace         left_brace          ,
    RightBrace        right_brace         ,
    Semicolon         semicolon           ,
    Apostrophe        apostrophe          ,
    Grave             grave               ,
    Comma             comma               ,
    Dot               dot                 ,
    Slash             slash               ,
    Backslash         backslash           ,

    LCtrl             l_ctrl              ,
    RCtrl             r_ctrl              ,
    LShift            l_shift             ,
    RShift            r_shift             ,
    LAlt              l_alt               ,
    RAlt              r_alt               ,
    LSystem           l_system            ,
    RSystem           r_system            ,
    LMeta             l_meta              ,
    RMeta             r_meta              ,
    Compose           compose             ,

    Home              home                ,
    End               end                 ,

    Up                up                  ,
    Down              down                ,
    Left              left                ,
    Right             right               ,

    PageUp            page_up             ,
    PageDown          page_down           ,

    Insert            insert              ,
    Delete            delete              ,

    SysRQ             sysrq               ,
    LineFeed          LineFeed            ,

    Kp0               kp_0                ,
    Kp1               kp_1                ,
    Kp2               kp_2                ,
    Kp3               kp_3                ,
    Kp4               kp_4                ,
    Kp5               kp_5                ,
    Kp6               kp_6                ,
    Kp7               kp_7                ,
    Kp8               kp_8                ,
    Kp9               kp_9                ,
    KpPlus            kp_plus             ,
    KpMinus           kp_minus            ,
    KpAsterisk        kp_asterisk         ,
    KpSlash           kp_slash            ,
    KpDot             kp_dot              ,
    KpEnter           kp_enter            ,
    KpEqual           kp_equal            ,
    KpComma           kp_comma            ,

    Mute              mute                ,
    VolumeDown        volume_down         ,
    VolumeUp          volume_up           ,
    NextTrack         next_track          ,
    PrevTrack         prev_track          ,
    PlayPause         play_pause          ,
    Stop              stop                ,
                                       
    BrowserBack       browser_back        ,
    BrowserForward    browser_forward     ,
    BrowserRefresh    browser_refresh     ,
    BrowserStop       browser_stop        ,
    BrowserSearch     browser_search      ,
    BrowserFavorites  browser_favorites   ,
    BrowserHome       browser_home        ,
                      
    LaunchMail        launch_mail         ,
    LaunchMediaSelect launch_media_select ,
    LaunchApp1        launch_app1         ,
    LaunchApp2        launch_app2         ,

    Power             power               ,
    Sleep             sleep               ,
    Menu              menu                ,
    Pause             pause               ,
    Snapshot          snapshot            ,
    Select            select              ,
    Print             print               ,
    Execute           execute             ,
    Help              help                ,
    Apps              apps                ,
                      
    OemPlus           oem_plus            ,
    OemComma          oem_comma           ,
    OemMinus          oem_minus           ,
    OemPeriod         oem_period          ,

    ZenkakuHankaku    zenkaku_hankaku     ,
    Katakana          katakana            ,
    Hiragana          hiragana            ,
    Henkan            henkan              ,
    KatakanaHiragana  katakana_hiragana   ,
    Muhenkan          muhenkan            ,

    Hangul            hangul              ,
    Hanja             hanja               ,
    Yen               yen                 ,

    Junja             junja               ,
    Final             final_              ,
    Kanji             kanji               ,                                
}

