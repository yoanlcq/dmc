use std::time::Instant;
use std::ops::Range;
use std::rc::Rc;
use super::{Vec2, Vec3};
use os::{
    OsKeyboardState,
    OsKeyboard,
    OsMouse,
    OsController,
    OsTablet,
    OsTouch,
};

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum Bus {
    Pci,
    Usb,
    Hil,
    Bluetooth,
    Virtual,
}

// A Context-specific unique ID for this device, which is useful as keys
// in collections.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct HidToken(pub u32);

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct HidProductInfo {
    pub vendor_id: u16,
    pub product_id: u16, // NOTE: ID_MODEL_ID on udev
    pub vendor_name: String,
    pub product_name: String, // NOTE: ID_MODEL on udev
}

#[derive(Debug, Clone, PartialEq)]
pub struct HidInfo {
    pub token: HidToken,
    pub name: String,
    // pub serial: Option<String>,
    pub product_info: Option<HidProductInfo>,
    //pub guid: Option<Guid>,
    pub plug_time: Instant,
    pub bus: Bus,
    //pub driver_version: Option<Semver>,
}

pub type AxisValue = i32;

// Taken from Linux's input_absinfo struct.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct AbsAxis1DInfo {
    pub range: Range<AxisValue>,
    pub dead_zone: Range<AxisValue>,
    pub resolution: u32, // In units per mm, or units per radian.
    pub fuzz: AxisValue,
}
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct AbsAxis2DInfo {
    pub range: Range<Vec2<AxisValue>>,
    pub dead_zone: Range<Vec2<AxisValue>>,
    pub resolution: Vec2<u32>, // In units per mm, or units per radian.
    pub fuzz: Vec2<AxisValue>,
}
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct AbsAxis3DInfo {
    pub range: Range<Vec3<AxisValue>>,
    pub dead_zone: Range<Vec3<AxisValue>>,
    pub resolution: Vec3<u32>, // In units per mm, or units per radian.
    pub fuzz: Vec3<AxisValue>,
}



#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Axis1DState {
    pub value: AxisValue,
    pub info: AbsAxis1DInfo,
}
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Axis2DState {
    pub value: Vec2<AxisValue>,
    pub info: AbsAxis2DInfo,
}
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Axis3DState {
    pub value: Vec3<AxisValue>,
    pub info: AbsAxis3DInfo,
}

fn axis_normalize_single(x: AxisValue, min: AxisValue, max: AxisValue) -> f64 {
    if x < 0 {
        x as f64 / min as f64
    } else {
        x as f64 / max as f64
    }
}

impl Axis1DState {
    pub fn normalized(&self) -> f64 {
        axis_normalize_single(self.value, self.info.range.start, self.info.range.end)
    }
}
impl Axis2DState {
    pub fn normalized(&self) -> Vec2<f64> {
        let x = axis_normalize_single(self.value.x, self.info.range.start.x, self.info.range.end.x);
        let y = axis_normalize_single(self.value.y, self.info.range.start.y, self.info.range.end.y);
        Vec2 { x, y }
    }
}
impl Axis3DState {
    pub fn normalized(&self) -> Vec3<f64> {
        let x = axis_normalize_single(self.value.x, self.info.range.start.x, self.info.range.end.x);
        let y = axis_normalize_single(self.value.y, self.info.range.start.y, self.info.range.end.y);
        let z = axis_normalize_single(self.value.z, self.info.range.start.z, self.info.range.end.z);
        Vec3 { x, y, z }
    }
}



pub trait Hid {
    fn info(&self) -> &HidInfo;
    fn is_connected(&self) -> bool;
}


//
// MOUSE
//


#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
    Side,
    Task,
    Forward,
    Back,
    Extra(u32),
}


#[derive(Debug)]
pub struct Mouse(pub(crate) OsMouse);

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct MouseState {
    pub abs_position: Vec2<u32>,
    pub left: bool,
    pub middle: bool,
    pub right: bool,
    pub side: bool,
    pub task: bool,
    pub forward: bool,
    pub back: bool,
}

impl Hid for Mouse {
    fn info(&self) -> &HidInfo { self.0.info() }
    fn is_connected(&self) -> bool { self.0.is_connected() }
}

impl Mouse {
    pub fn query_state(&self) -> Result<MouseState, ()> {
        self.0.query_state()
    }
    pub fn warp_absolute(&self, p: Vec2<u32>) -> Result<(), ()> {
        self.0.warp_absolute(p)
    }
}


//
// KEYBOARD
//


#[derive(Debug)]
pub struct Keyboard(pub(crate) OsKeyboard);

macro_rules! vkeys {
    ($($VKey:ident $vkey:ident,)+) => {
        /*
        /// `VKey` means "Virtual Key", that is, the raw value reported by the
        /// OS which corresponds to a standard QWERTY keyboard.
        ///
        /// The SDL2 equivalent is `SDL_Scancode`.
        /// There is no X11 equivalent, because `Keycode` values are
        /// driver-specific. They range from 8 to 255 but, unlike in Windows,
        /// do not have a "standard" meaning: they are only meaningful
        /// when converted to `Keysym`s.
        ///
        /// These are not to be confused with "physical keys", which
        /// are the result of translating a virtual key code according to
        /// the actual keyboard's layout.
        */
        //#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
        pub type OsScancode = u8;

        /// A `Key` is understood as a "physical key", that is, the value
        /// that indicates the actual meaning of the key with regards to the
        /// user's keyboard layout.
        ///
        /// The SDL2 equivalent is `SDL_Keycode`.
        /// The X11 equivalent is `Keysym`.
        ///
        /// These are NOT appropriate for text input. For this, read the
        /// `text` member of the `KeyboardKeyPressed` event instead.
        #[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
        pub enum Key {
            $($VKey),+,
            Other(u64),
        }
        /// Most platforms provide a (supposedly) efficient way to query
        /// the whole keyboard's state in a single call.
        ///
        /// Under Windows, it's `GetKeyboardState()`.
        /// Under X11, it's `XQueryKeymap()`.
        #[derive(Debug)]
        pub struct KeyboardState(pub(crate) OsKeyboardState);
            // $(pub $vkey: bool),+
    };
}

vkeys!{
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

impl Hid for Keyboard {
    fn info(&self) -> &HidInfo { self.0.info() }
    fn is_connected(&self) -> bool { self.0.is_connected() }
}

// WISH: get toggle state (e.g is Caps Lock on or off?)
impl Keyboard {
    pub fn query_state(&self) -> Result<KeyboardState, ()> {
        self.0.query_state()
    }
    pub fn query_key_state(&self, key: Key) -> Result<bool, ()> {
        self.0.query_key_state(key)
    }
    pub fn key_from_scancode(&self, scancode: OsScancode) -> Result<Key, ()> {
        self.0.key_from_scancode(scancode)
    }
    pub fn scancode_from_key(&self, key: Key) -> Result<OsScancode, ()> {
        self.0.scancode_from_key(key)
    }
    pub fn query_key_name(&self, key: Key) -> Result<String, ()> {
        self.0.query_key_name(key)
    }
}

impl KeyboardState {
    pub fn key(&self, scancode: OsScancode) -> bool {
        self.0.key(scancode)
    }
}


//
// CONTROLLER (Gamepads, Joysticks, Steering wheels, etc)
//

// NOTE: from udev sources
// https://github.com/systemd/systemd
// 'ID_INPUT'
// 'ID_INPUT_ACCELEROMETER'
// 'ID_INPUT_JOYSTICK'
// 'ID_INPUT_KEY'
// 'ID_INPUT_KEYBOARD'
// 'ID_INPUT_MOUSE'
// 'ID_INPUT_POINTINGSTICK'
// 'ID_INPUT_SWITCH'
// 'ID_INPUT_TABLET'
// 'ID_INPUT_TABLET_PAD'
// 'ID_INPUT_TOUCHPAD'
// 'ID_INPUT_TOUCHSCREEN'
// 'ID_INPUT_TRACKBALL'
//
// Detecting the kind :
// BTN_JOYSTICK
// BTN_GAMEPAD
// ABS_WHEEL (look at how evtest detects it)
//
// Very interesting:
// https://www.kernel.org/doc/html/v4.12/input/gamepad.html


#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum GamepadModel {
    Xbox360,
    XboxOne,
    Generic,
    // DualShock...
}
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum ControllerKind {
    Gamepad(GamepadModel),
    Joystick,
    SteeringWheel,
}

macro_rules! controller_items {
    (
        buttons {$($Button:ident $button:ident,)+}
        numbered_buttons {$($NumberedButton:ident $numbered_button:ident,)+}
        axes_1d {$($Axis1D:ident $axis_1d:ident,)+}
        axes_2d {$($Axis2D:ident $axis_2d:ident,)+}
        axes_3d {$($Axis3D:ident $axis_3d:ident,)+}
    ) => {
        #[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
        pub enum ControllerButton {
            $($Button,)+
            $($NumberedButton(u32),)+
            Other(u32),
        }
        #[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
        pub enum ControllerAxis1D {
            $($Axis1D,)+
            Other(u32),
        }
        #[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
        pub enum ControllerAxis2D {
            $($Axis2D,)+
            Other(u32),
        }
        #[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
        pub enum ControllerAxis3D {
            $($Axis3D,)+
            Other(u32),
        }
        #[derive(Debug, Clone, Hash, PartialEq, Eq)]
        pub struct ControllerFeatures {
            $($button: bool,)+
            $($numbered_button: bool,)+
            $($axis_1d: bool,)+
            $($axis_2d: bool,)+
            $($axis_3d: bool,)+
            // + TODO WISH: Haptics here
        }
        #[derive(Debug, Clone, Hash, PartialEq, Eq)]
        pub struct ControllerState {
            $($button: bool,)+
            $($numbered_button: [bool; 8],)+
            $($axis_1d: Axis1DState,)+
            $($axis_2d: Axis2DState,)+
            $($axis_3d: Axis3DState,)+
        }
    };
}

controller_items!{
    buttons {
        // Gamepads
        X x, Y y, Z z, A a, B b, C c,
        LShoulder l_shoulder, LShoulder2 l_shoulder2, 
        RShoulder r_shoulder, RShoulder2 r_shoulder2, 
        LStickClick l_stick_click,
        RStickClick R_stick_click,
        Select select, Start start, Mode mode,
        // Joystick
        Trigger trigger, Pinkie pinkie, Dead dead,
    }
    numbered_buttons {
        // Joystick
        Thumb thumb, Top top, Base base,
    }
    axes_1d {
        // Xbox360 gamepads
        LTrigger l_trigger, RTrigger r_trigger,
        // Steering Wheel pedals
        Throttle throttle, Rudder rudder, Wheel wheel, Gas gas, Brake brake,
    }
    axes_2d {
        // Gamepads
        LStick l_stick, RStick r_stick, Dpad dpad, 
    }
    axes_3d {
        // Joysticks
        MainJoystick main_joystick,
    }
}



#[derive(Debug)]
pub struct Controller(pub(crate) OsController);

impl Hid for Controller {
    fn info(&self) -> &HidInfo { self.0.info() }
    fn is_connected(&self) -> bool { self.0.is_connected() }
}

impl Controller {
    pub fn kind(&self) -> ControllerKind { self.0.kind() }
    pub fn features(&self) -> ControllerFeatures { self.0.features() }
    pub fn query_state(&self) -> Result<ControllerState, ()> {
        self.0.query_state()
    }
    pub fn query_button (&self, btn : ControllerButton) -> Result<bool, ()> { self.0.query_button(btn) }
    pub fn query_1d_axis(&self, axis: ControllerAxis1D) -> Result<Axis1DState, ()> { self.0.query_1d_axis(axis) }
    pub fn query_2d_axis(&self, axis: ControllerAxis2D) -> Result<Axis2DState, ()> { self.0.query_2d_axis(axis) }
    pub fn query_3d_axis(&self, axis: ControllerAxis3D) -> Result<Axis3DState, ()> { self.0.query_3d_axis(axis) }
}


//
// PEN TABLETS
//

// TODO:
// - Refine this when implementing with WinTab
// TODO
// FingerwheelMotion,
// 4DMouseThumbwheel
// 4DMouseRotation

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct TabletFeatures {
    pad_button_count: u32,
    stylus_button_count: u32,
    pressure: AbsAxis1DInfo,
    tilt: AbsAxis2DInfo,
    raw_position: AbsAxis2DInfo,
}

/*
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum StylusDevice {
    Regular,
    ArtPen,
    Airbrush,
    FourDMouse,
    FiveButtonPuck,
}
*/

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum ToolType {
    Pen, Eraser,
}
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct TabletState {
    pad_buttons: [bool; 32],
    stylus_buttons: [bool; 2],
    pressure: Axis1DState,
    tilt: Axis2DState,
    abs_position: Axis2DState,
    raw_position: Axis2DState,
}

#[derive(Debug)]
pub struct Tablet(pub(crate) OsTablet);

impl Hid for Tablet {
    fn info(&self) -> &HidInfo { self.0.info() }
    fn is_connected(&self) -> bool { self.0.is_connected() }
}

impl Tablet {
    pub fn features(&self) -> TabletFeatures { self.0.features() }
    pub fn query_state(&self) -> Result<TabletState, ()> { self.0.query_state() }
}

//
// TOUCH
//

// TODO
#[derive(Debug)]
pub struct Touch(pub(crate) OsTouch);

