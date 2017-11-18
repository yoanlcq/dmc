use std::time::Instant;
use std::ops::Range;
use std::rc::Rc;
use vek::{Vec2, Vec3};
use backend::{BackendContext, BackendHid};

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

#[derive(Debug, Clone, PartialEq)]
pub struct HidInfo {
    pub token: HidToken,

    pub name: String,
    pub product_name: String, // NOTE: ID_MODEL on udev
    pub product_id: u16, // NOTE: ID_MODEL_ID on udev
    pub vendor_name: String,
    pub vendor_id: u16,
    pub serial: String,
    pub bus: Bus,
    pub plug_time: Instant,

    //pub guid: Option<Guid>,
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
    pub info: AbsAxis1DInfo,
}
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Axis3DState {
    pub value: Vec3<AxisValue>,
    pub info: AbsAxis1DInfo,
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
pub struct Mouse {
    internal: BackendHid,
    backend: Rc<BackendContext>,
    info: HidInfo,
    // some_extra_mousey_info: Foo,
}

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
    fn info(&self) -> &HidInfo { &self.info }
    fn is_connected(&self) -> bool { self.internal.is_connected() }
}

impl Mouse {
    pub fn query_state(&self) -> Result<MouseState, ()> {
        unimplemented!{}
    }
    pub fn warp_absolute(&self, p: Vec2<u32>) -> Result<(), ()> {
        unimplemented!{}
    }
}


//
// KEYBOARD
//


#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct Key;

#[derive(Debug)]
pub struct Keyboard {
    internal: BackendHid,
    backend: Rc<BackendContext>,
    info: HidInfo,
    // some_extra_keyboardey_info: Foo,
}

macro_rules! vkeys {
    ($($VKey:ident $vkey:ident,)+) => {
        #[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
        pub enum VKey {
            $($VKey),+
        }
        #[derive(Debug, Clone, Hash, PartialEq, Eq)]
        pub struct KeyboardState {
            $($vkey: bool),+
        }
    };
}

vkeys!{
    Num1             num1            ,
    Num2             num2            ,
    Num3             num3            ,
    Num4             num4            ,
    Num5             num5            ,
    Num6             num6            ,
    Num7             num7            ,
    Num8             num8            ,
    Num9             num9            ,
    Num0             num0            ,
    A                a               ,
    B                b               ,
    C                c               ,
    D                d               ,
    E                e               ,
    F                f               ,
    G                g               ,
    H                h               ,
    I                i               ,
    J                j               ,
    K                k               ,
    L                l               ,
    M                m               ,
    N                n               ,
    O                o               ,
    P                p               ,
    Q                q               ,
    R                r               ,
    S                s               ,
    T                t               ,
    U                u               ,
    V                v               ,
    W                w               ,
    X                x               ,
    Y                y               ,
    Z                z               ,
    F1               f1              ,
    F2               f2              ,
    F3               f3              ,
    F4               f4              ,
    F5               f5              ,
    F6               f6              ,
    F7               f7              ,
    F8               f8              ,
    F9               f9              ,
    F10              f10             ,
    F11              f11             ,
    F12              f12             ,

    Esc              esc             ,
    Space            space           ,
    Backspace        backspace       ,
    Tab              tab             ,
    Enter            enter           ,

    CapsLock         caps_lock       ,
    NumLock          num_lock        ,
    ScrollLock       scroll_lock     ,

    Minus            minus           ,
    Equal            equal           ,
    LeftBrace        left_brace      ,
    RightBrace       right_brace     ,
    Semicolon        semicolon       ,
    Apostrophe       apostrophe      ,
    Grave            grave           ,
    Comma            comma           ,
    Dot              dot             ,
    Slash            slash           ,
    Backslash        backslash       ,

    LCtrl            l_ctrl          ,
    RCtrl            r_ctrl          ,
    LShift           l_shift         ,
    RShift           r_shift         ,
    LAlt             l_alt           ,
    RAlt             r_alt           ,
    LSystem          l_system        ,
    RSystem          r_system        ,
    LMeta            l_meta          ,
    RMeta            r_meta          ,
    Compose          compose         ,

    ScrollUp         scrollup        ,
    ScrollDown       scrolldown      ,

    Home             home            ,
    End              end             ,

    Up               up              ,
    Down             down            ,
    Left             left            ,
    Right            right           ,

    PageUp           page_up         ,
    PageDown         page_down       ,

    Insert           insert          ,
    Delete           delete          ,

    SysRQ            sysrq           ,
    LineFeed         LineFeed        ,

    Kp0              kp_0            ,
    Kp1              kp_1            ,
    Kp2              kp_2            ,
    Kp3              kp_3            ,
    Kp4              kp_4            ,
    Kp5              kp_5            ,
    Kp6              kp_6            ,
    Kp7              kp_7            ,
    Kp8              kp_8            ,
    Kp9              kp_9            ,
    KpPlus           kp_plus         ,
    KpMinus          kp_minus        ,
    KpAsterisk       kp_asterisk     ,
    KpSlash          kp_slash        ,
    KpDot            kp_dot          ,
    KpEnter          kp_enter        ,
    KpLeftParen      kp_left_paren   ,
    KpRightParen     kp_right_paren  ,
    KpEqual          kp_equal        ,
    KpPlusMinus      kp_plus_minus   ,
    KpComma          kp_comma        ,

    Macro            macro_          ,
    Mute             mute            ,
    Volumedown       volumedown      ,
    Volumeup         volumeup        ,
    Power            power           ,
    Pause            pause           ,
    Scale            scale           ,

    Zenkakuhankaku   zenkakuhankaku  ,
    _102nd           _102nd          ,
    Ro               ro              ,
    Katakana         katakana        ,
    Hiragana         hiragana        ,
    Henkan           henkan          ,
    Katakanahiragana katakanahiragana,
    Muhenkan         muhenkan        ,
    KpJpComma        kp_jp_comma     ,

    Hangeul          hangeul         ,
    Hanja            hanja           ,
    Yen              yen             ,
}

impl Hid for Keyboard {
    fn info(&self) -> &HidInfo { &self.info }
    fn is_connected(&self) -> bool { self.internal.is_connected() }
}

impl Keyboard {
    pub fn query_state(&self) -> Result<KeyboardState, ()> {
        unimplemented!{}
    }
    pub fn query_vkey_state(&self, vkey: VKey) -> Result<bool, ()> {
        unimplemented!{}
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
pub struct Controller {
    internal: BackendHid,
    backend: Rc<BackendContext>,
    info: HidInfo,
    features: ControllerFeatures,
    kind: ControllerKind,
    // some_extra_gamepadey_info: Foo,
}

impl Hid for Controller {
    fn info(&self) -> &HidInfo { &self.info }
    fn is_connected(&self) -> bool { self.internal.is_connected() }
}

impl Controller {
    pub fn kind(&self) -> ControllerKind { unimplemented!{} }
    pub fn features(&self) -> ControllerFeatures { unimplemented!{} }
    pub fn query_state(&self) -> Result<ControllerState, ()> {
        unimplemented!{}
    }
    pub fn query_button (&self, btn : ControllerButton) -> Result<bool, ()> { unimplemented!{} }
    pub fn query_1d_axis(&self, axis: ControllerAxis1D) -> Result<Axis1DState, ()> { unimplemented!{} }
    pub fn query_2d_axis(&self, axis: ControllerAxis2D) -> Result<Axis2DState, ()> { unimplemented!{} }
    pub fn query_3d_axis(&self, axis: ControllerAxis3D) -> Result<Axis3DState, ()> { unimplemented!{} }
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
pub struct PenTabletFeatures {
    pad_button_count: u32,
    stylus_button_count: u32,
    pressure: AbsAxis1DInfo,
    tilt: Vec2<AbsAxis1DInfo>,
    raw_position: Vec2<AbsAxis1DInfo>,
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
pub struct PenTabletState {
    pad_buttons: [bool; 32],
    stylus_buttons: [bool; 2],
    pressure: Axis1DState,
    tilt: Axis2DState,
    abs_position: Axis2DState,
    raw_position: Axis2DState,
}

#[derive(Debug)]
pub struct PenTablet {
    internal: BackendHid,
    backend: Rc<BackendContext>,
    info: HidInfo,
    features: PenTabletFeatures,
    // some_extra_info: Foo,
}

impl Hid for PenTablet {
    fn info(&self) -> &HidInfo { &self.info }
    fn is_connected(&self) -> bool { self.internal.is_connected() }
}

impl PenTablet {
    pub fn features(&self) -> PenTabletFeatures { unimplemented!{} }
    pub fn query_state(&self) -> Result<PenTabletState, ()> { unimplemented!{} }
}

//
// TOUCH
//

// TODO
#[derive(Debug)]
pub struct Touch;

