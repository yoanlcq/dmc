use super::x11::xlib as x;
use super::x11::keysym;
use device::keyboard::Keysym;

macro_rules! x_keysyms {
    ($($Key:ident $XK:ident,)+ { ignored: $($Ignored:ident,)* }) => {
        impl Keysym {
            pub(crate) fn x_keysym(&self) -> Option<x::KeySym> {
                match *self {
                    $(Keysym::$Key => Some(keysym::$XK as _),)+
                    $(Keysym::$Ignored => None,)*
                    Keysym::Other(x) => Some(x as _),
                }
            }
            pub(crate) fn from_x_keysym(x_keysym: x::KeySym) -> Self {
                match x_keysym as _ {
                    $(keysym::$XK => Keysym::$Key,)+
                    x => Keysym::Other(x as _),
                }
            }
        }
    };
}

x_keysyms!{
    Num1             XK_1            ,
    Num2             XK_2            ,
    Num3             XK_3            ,
    Num4             XK_4            ,
    Num5             XK_5            ,
    Num6             XK_6            ,
    Num7             XK_7            ,
    Num8             XK_8            ,
    Num9             XK_9            ,
    Num0             XK_0            ,
    A                XK_a               ,
    B                XK_b               ,
    C                XK_c               ,
    D                XK_d               ,
    E                XK_e               ,
    F                XK_f               ,
    G                XK_g               ,
    H                XK_h               ,
    I                XK_i               ,
    J                XK_j               ,
    K                XK_k               ,
    L                XK_l               ,
    M                XK_m               ,
    N                XK_n               ,
    O                XK_o               ,
    P                XK_p               ,
    Q                XK_q               ,
    R                XK_r               ,
    S                XK_s               ,
    T                XK_t               ,
    U                XK_u               ,
    V                XK_v               ,
    W                XK_w               ,
    X                XK_x               ,
    Y                XK_y               ,
    Z                XK_z               ,
    F1               XK_F1              ,
    F2               XK_F2              ,
    F3               XK_F3              ,
    F4               XK_F4              ,
    F5               XK_F5              ,
    F6               XK_F6              ,
    F7               XK_F7              ,
    F8               XK_F8              ,
    F9               XK_F9              ,
    F10              XK_F10             ,
    F11              XK_F11             ,
    F12              XK_F12             ,

    Esc              XK_Escape             ,
    Space            XK_space           ,
    Backspace        XK_BackSpace       ,
    Tab              XK_Tab             ,
    Enter            XK_Return           ,

    CapsLock         XK_Caps_Lock       ,
    NumLock          XK_Num_Lock        ,
    ScrollLock       XK_Scroll_Lock     ,

    Minus            XK_minus           ,
    Equal            XK_equal           ,
    LeftBrace        XK_braceleft     ,
    RightBrace       XK_braceright     ,
    Semicolon        XK_semicolon       ,
    Apostrophe       XK_apostrophe      ,
    Grave            XK_grave           ,
    Comma            XK_comma           ,
    Dot              XK_period          ,
    Slash            XK_slash           ,
    Backslash        XK_backslash       ,

    LCtrl            XK_Control_L          ,
    RCtrl            XK_Control_R          ,
    LShift           XK_Shift_L         ,
    RShift           XK_Shift_R         ,
    LAlt             XK_Alt_L           ,
    RAlt             XK_Alt_R           ,
    LSystem          XK_Super_L        ,
    RSystem          XK_Super_R        ,
    LMeta            XK_Meta_L         ,
    RMeta            XK_Meta_R         ,
    Compose          XK_Multi_key         ,

    Home             XK_Home            ,
    End              XK_End             ,

    Up               XK_Up              ,
    Down             XK_Down            ,
    Left             XK_Left            ,
    Right            XK_Right           ,

    PageUp           XK_Prior ,
    PageDown         XK_Next  ,

    Insert           XK_Insert          ,
    Delete           XK_Delete          ,

    SysRQ            XK_Sys_Req           ,
    LineFeed         XK_Linefeed        ,

    Kp0              XK_KP_0            ,
    Kp1              XK_KP_1            ,
    Kp2              XK_KP_2            ,
    Kp3              XK_KP_3            ,
    Kp4              XK_KP_4            ,
    Kp5              XK_KP_5            ,
    Kp6              XK_KP_6            ,
    Kp7              XK_KP_7            ,
    Kp8              XK_KP_8            ,
    Kp9              XK_KP_9            ,
    KpPlus           XK_KP_Add         ,
    KpMinus          XK_KP_Subtract        ,
    KpAsterisk       XK_KP_Multiply     ,
    KpSlash          XK_KP_Divide        ,
    KpDot            XK_KP_Decimal          ,
    KpEnter          XK_KP_Enter        ,
    KpEqual          XK_KP_Equal        ,
    KpComma          XK_KP_Separator        ,

    Mute             XF86XK_AudioMute            ,
    VolumeDown       XF86XK_AudioLowerVolume      ,
    VolumeUp         XF86XK_AudioRaiseVolume      ,
    Power            XF86XK_PowerOff           ,
    Pause            XK_Pause           ,

    ZenkakuHankaku   XK_Zenkaku_Hankaku  ,
    Katakana         XK_Katakana        ,
    Hiragana         XK_Hiragana        ,
    Henkan           XK_Henkan          ,
    KatakanaHiragana XK_Hiragana_Katakana,
    Muhenkan         XK_Muhenkan        ,

    Yen              XK_yen             ,

    {
        ignored:
        Hangul,
        Hanja,
        Junja,
        Final,
        Kanji,
        NextTrack, PrevTrack, PlayPause,
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
        Sleep,
        Menu,
        Snapshot,
        Select,
        Print,
        Execute,
        Help,
        Apps,
        Plus,
        Comma,
        Minus,
        Dot,
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
        ImeConvert,
        ImeNonConvert,
        ImeAccept,
        ImeModeChange,
    }
}
