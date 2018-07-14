use std::slice;
use std::ptr;
use std::mem;
use std::os::raw::c_int;
use std::collections::HashMap;
use error::{Result, failed};
use super::x11::xinput2 as xi2;
use super::x11::xlib as x;
use super::xlib_error;
use super::X11SharedContext;
use super::atoms::PreloadedAtoms;
use device::{
    self,
    DeviceID, DeviceInfo, ButtonState, UsbIDs, Bus, AxisInfo,
    ControllerButton, ControllerAxis, ControllerState, ControllerInfo,
    VibrationState,
    KeyboardInfo, KeyState, KeyboardState, Keysym, Keycode,
    MouseInfo, MouseState, MouseButton,
    TabletInfo, TabletState, TabletPadButton, TabletStylusButton,
    TouchInfo,
};
use Vec2;

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum X11DeviceID {
    CoreKeyboard,
    CorePointer,
    XISlave(c_int),
}

impl X11DeviceID {
    pub fn xi_device_id(&self) -> device::Result<c_int> {
        match *self {
            X11DeviceID::XISlave(x) => Ok(x),
            _ => device::failed("This device ID is not a XI device ID"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct X11TabletInfo;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct X11KeyboardState;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct X11MouseButtonsState;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct X11TabletPadButtonsState;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct X11TabletStylusButtonsState;

impl X11TabletInfo {
    pub fn pressure_axis(&self) -> &AxisInfo { unimplemented!{} }
    pub fn tilt_axis(&self) -> Vec2<&AxisInfo> { unimplemented!{} }
    pub fn physical_position_axis(&self) -> &AxisInfo { unimplemented!{} }
}


impl X11KeyboardState {
    pub fn keycode(&self, key: Keycode) -> Option<KeyState> {
        unimplemented!{}
    }
    pub fn keysym(&self, key: Keysym) -> Option<KeyState> {
        unimplemented!{}
    }
}
impl X11MouseButtonsState {
    pub fn button(&self, button: MouseButton) -> Option<ButtonState> {
        unimplemented!{}
    }
}
impl X11TabletPadButtonsState {
    pub fn button(&self, button: TabletPadButton) -> Option<ButtonState> {
        unimplemented!{}
    }
}
impl X11TabletStylusButtonsState {
    pub fn button(&self, button: TabletStylusButton) -> Option<ButtonState> {
        unimplemented!{}
    }
}

impl X11SharedContext {
    pub fn keyboard_state(&self, keyboard: X11DeviceID) -> device::Result<KeyboardState> {
        /*
        let x_display = self.lock_x_display();
        // Quoting the man page:
        // Byte N (from 0) contains the bits for keys 8N to 8N + 7 with the least significant bit in the byte representing key 8N.
        let mut key_bits: [u8; 32] = [0; 32];
        let _ = xlib_error::sync_catch(*x_display, || unsafe {
            x::XQueryKeymap(*x_display, key_bits.as_mut_ptr() as _)
        })?;
        unimplemented!{} // FIXME: We're completely ignoring the keyboard ID :(
        */
        // FIXME: Do it properly using our stored device infos
        unimplemented!{}
    }
    pub fn keyboard_keycode_state(&self, keyboard: X11DeviceID, keycode: Keycode) -> device::Result<KeyState> {
        unimplemented!{}
    }
    pub fn keyboard_keysym_state(&self, keyboard: X11DeviceID, keysym: Keysym) -> device::Result<KeyState> {
        unimplemented!{}
    }
    pub fn keysym_name(&self, keysym: Keysym) -> device::Result<String> {
        unimplemented!{}
    }
    pub fn keysym_from_keycode(&self, keyboard: X11DeviceID, keycode: Keycode) -> device::Result<Keysym> {
        unimplemented!{}
    }
    pub fn keycode_from_keysym(&self, keyboard: X11DeviceID, keysym: Keysym) -> device::Result<Keycode> {
        unimplemented!{}
    }
    pub fn mouse_state(&self, mouse: X11DeviceID) -> device::Result<MouseState> {
        unimplemented!{}
    }
    pub fn tablet_state(&self, tablet: X11DeviceID) -> device::Result<TabletState> {
        unimplemented!{}
    }
}


#[derive(Debug, Clone)]
pub struct XI2DeviceInfo {
    pub name: String,
    pub role: Option<XI2DeviceRole>,
    /// If the device is a master pointer or a master keyboard, attachment
    /// specifies the paired master keyboard, or the paired master pointer,
    /// respectively.  If the device is a non-floating slave device
    /// attachment specifies the master device this device is attached to.
    /// If the device is a floating slave, attachment is undefined.
    pub attachment: c_int,
    pub is_enabled: bool,
    pub key_class: Option<XI2KeyClassInfo>,
    pub button_class: Option<XI2ButtonClassInfo>,
    pub touch_classes: Vec<XI2TouchClassInfo>,
    // These map a valuator index to the appropriate class, if any.
    // "scroll class" is optional, and always backed by a "valuator class".
    pub valuator_classes: HashMap<usize, XI2ValuatorClassInfo>,
    pub scroll_classes: HashMap<usize, XI2ScrollClassInfo>,
}

impl XI2DeviceInfo {
    pub fn replace_classes<I: IntoIterator<Item=XI2DeviceAnyClassInfo>>(&mut self, classes: I, atoms: &PreloadedAtoms) {
        self.key_class = None;
        self.button_class = None;
        self.touch_classes.clear();
        self.valuator_classes.clear();
        self.scroll_classes.clear();

        for class in classes.into_iter() {
            match class {
                XI2DeviceAnyClassInfo::Key(class) => {
                    assert!(self.key_class.is_none()); // Spec says there's not more than one per device
                    self.key_class = Some(XI2KeyClassInfo::from_xi2(&class));
                },
                XI2DeviceAnyClassInfo::Button(class) => {
                    assert!(self.button_class.is_none()); // Spec says there's not more than one per device
                    self.button_class = Some(XI2ButtonClassInfo::from_xi2(&class, atoms));
                },
                XI2DeviceAnyClassInfo::Touch(class) => {
                    self.touch_classes.push(XI2TouchClassInfo::from_xi2(&class));
                },
                XI2DeviceAnyClassInfo::Valuator(class) => {
                    assert!(class.number >= 0);
                    self.valuator_classes.insert(class.number as _, XI2ValuatorClassInfo::from_xi2(&class, atoms));
                },
                XI2DeviceAnyClassInfo::Scroll(class) => {
                    assert!(class.number >= 0);
                    self.scroll_classes.insert(class.number as _, XI2ScrollClassInfo::from_xi2(&class));
                },
            }
        }
    }
}


#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum XI2ButtonLabel {
    Left           ,
    Middle         ,
    Right          ,
    Side           ,
    Extra          ,
    Forward        ,
    Back           ,
    Task           ,
    Unknown        ,
    WheelUp        ,
    WheelDown      ,
    HorizWheelLeft ,
    HorizWheelRight,
    Other(x::Atom),
}

impl XI2ButtonLabel {
    pub fn from_xi2(label: x::Atom, atoms: &PreloadedAtoms) -> Option<Self> {
        if label == 0 {
            return None;
        }
        Some([
            (atoms.Button_Left             (), XI2ButtonLabel::Left           ),
            (atoms.Button_Middle           (), XI2ButtonLabel::Middle         ),
            (atoms.Button_Right            (), XI2ButtonLabel::Right          ),
            (atoms.Button_Side             (), XI2ButtonLabel::Side           ),
            (atoms.Button_Extra            (), XI2ButtonLabel::Extra          ),
            (atoms.Button_Forward          (), XI2ButtonLabel::Forward        ),
            (atoms.Button_Back             (), XI2ButtonLabel::Back           ),
            (atoms.Button_Task             (), XI2ButtonLabel::Task           ),
            (atoms.Button_Unknown          (), XI2ButtonLabel::Unknown        ),
            (atoms.Button_Wheel_Up         (), XI2ButtonLabel::WheelUp        ),
            (atoms.Button_Wheel_Down       (), XI2ButtonLabel::WheelDown      ),
            (atoms.Button_Horiz_Wheel_Left (), XI2ButtonLabel::HorizWheelLeft ),
            (atoms.Button_Horiz_Wheel_Right(), XI2ButtonLabel::HorizWheelRight),
        ].iter()
            .filter_map(|(k, v)| k.as_ref().ok().map(|k| if label == *k { Some(*v) } else { None }))
            .filter_map(|x| x)
            .next()
            .unwrap_or(XI2ButtonLabel::Other(label)))
    }
}


#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct XI2ButtonClassInfo {
    pub button_labels: Vec<Option<XI2ButtonLabel>>,
}

impl XI2ButtonClassInfo {
    pub fn from_xi2(info: &xi2::XIButtonClassInfo, atoms: &PreloadedAtoms) -> Self {
        let &xi2::XIButtonClassInfo {
            _type: _, sourceid: _, num_buttons, labels, state
        } = info;

        Self {
            button_labels: unsafe {
                slice::from_raw_parts(labels, num_buttons as _)
            }.iter().map(|label| XI2ButtonLabel::from_xi2(*label, atoms)).collect(),
        }
    }
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum XI2AxisLabel {
    RelX           ,
    RelY           ,
    RelHorizScroll ,
    RelVertScroll  ,
    AbsMTTouchMajor,
    AbsMTPressure  ,
    AbsX           ,
    AbsY           ,
    AbsPressure    ,
    AbsTiltX       ,
    AbsTiltY       ,
    AbsWheel       ,
    Other(x::Atom),
}

impl XI2AxisLabel {
    pub fn from_xi2(label: x::Atom, atoms: &PreloadedAtoms) -> Option<Self> {
        if label == 0 {
            return None;
        }
        Some([
            (atoms.Rel_X             (), XI2AxisLabel::RelX           ),
            (atoms.Rel_Y             (), XI2AxisLabel::RelY           ),
            (atoms.Rel_Horiz_Scroll  (), XI2AxisLabel::RelHorizScroll ),
            (atoms.Rel_Vert_Scroll   (), XI2AxisLabel::RelVertScroll  ),
            (atoms.Abs_MT_Touch_Major(), XI2AxisLabel::AbsMTTouchMajor),
            (atoms.Abs_MT_Pressure   (), XI2AxisLabel::AbsMTPressure  ),
            (atoms.Abs_X             (), XI2AxisLabel::AbsX           ),
            (atoms.Abs_Y             (), XI2AxisLabel::AbsY           ),
            (atoms.Abs_Pressure      (), XI2AxisLabel::AbsPressure    ),
            (atoms.Abs_Tilt_X        (), XI2AxisLabel::AbsTiltX       ),
            (atoms.Abs_Tilt_Y        (), XI2AxisLabel::AbsTiltY       ),
            (atoms.Abs_Wheel         (), XI2AxisLabel::AbsWheel       ),
        ].iter()
            .filter_map(|(k, v)| k.as_ref().ok().map(|k| if label == *k { Some(*v) } else { None }))
            .filter_map(|x| x)
            .next()
            .unwrap_or(XI2AxisLabel::Other(label)))
    }
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum XI2AxisMode {
    Absolute,
    Relative,
}

impl XI2AxisMode {
    pub fn from_xi2(mode: c_int) -> Option<Self> {
        match mode {
            xi2::XIModeAbsolute => Some(XI2AxisMode::Absolute),
            xi2::XIModeRelative => Some(XI2AxisMode::Relative),
            _ => None,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct XI2AxisInfo {
    pub min: f64,
    pub max: f64,
    /// Resolution in counts/meter.
    pub resolution: i32,
    pub mode: XI2AxisMode,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct XI2ValuatorClassInfo {
    pub label: Option<XI2AxisLabel>,
    pub axis_info: XI2AxisInfo,
    pub value: f64,
}

impl XI2ValuatorClassInfo {
    pub fn from_xi2(info: &xi2::XIValuatorClassInfo, atoms: &PreloadedAtoms) -> Self {
        let &xi2::XIValuatorClassInfo {
            _type: _, sourceid: _, number, label, min, max, value, resolution, mode,
        } = info;

        Self {
            axis_info: XI2AxisInfo {
                min, max, resolution,
                mode: XI2AxisMode::from_xi2(mode).unwrap(),
            },
            label: XI2AxisLabel::from_xi2(label, atoms),
            value,
        }
    }
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum XI2ScrollType {
    Vertical,
    Horizontal,
}

impl XI2ScrollType {
    pub fn from_xi2(scroll_type: c_int) -> Option<Self> {
        match scroll_type {
            xi2::XIScrollTypeVertical => Some(XI2ScrollType::Vertical),
            xi2::XIScrollTypeHorizontal => Some(XI2ScrollType::Horizontal),
            _ => None,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct XI2ScrollClassInfo {
    pub scroll_type: XI2ScrollType,
    pub increment: f64,
    /// NoEmulation: no legacy scroll button events are generated for events on this scrolling axis.
    pub no_emulation: bool,
    /// Preferred: This axis is the preferred axis for emulating valuator events from legacy scroll button events.
    pub preferred: bool,
}

impl XI2ScrollClassInfo {
    pub fn from_xi2(info: &xi2::XIScrollClassInfo) -> Self {
        let &xi2::XIScrollClassInfo {
            _type: _, sourceid: _, number, scroll_type, increment, flags
        } = info;

        Self {
            scroll_type: XI2ScrollType::from_xi2(scroll_type).unwrap(),
            increment,
            no_emulation: (flags & xi2::XIScrollFlagNoEmulation) != 0,
            preferred: (flags & xi2::XIScrollFlagPreferred) != 0,
        }
    }
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum XI2TouchMode {
    /// 'DirectTouch':
    /// These devices map their input region to a subset of the screen region. Touch
    /// events are delivered to window at the location of the touch. "direct"
    /// here refers to the user manipulating objects at their screen location.
    /// An example of a DirectTouch device is a touchscreen.
    Direct,
    /// 'DependentTouch':
    /// These devices do not have a direct correlation between a touch location and
    /// a position on the screen. Touch events are delivered according to the
    /// location of the device's cursor and often need to be interpreted
    /// relative to the current position of that cursor. Such interactions are
    /// usually the result of a gesture performed on the device, rather than
    /// direct manipulation. An example of a DependentTouch device is a
    /// trackpad.
    Dependent,
}

impl XI2TouchMode {
    pub fn from_xi2(mode: c_int) -> Option<Self> {
        match mode {
            xi2::XIDependentTouch => Some(XI2TouchMode::Dependent),
            xi2::XIDirectTouch => Some(XI2TouchMode::Direct),
            _ => None,
        } 
    }
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct XI2TouchClassInfo {
    pub mode: XI2TouchMode,
    pub num_touches: usize,
}
impl XI2TouchClassInfo {
    pub fn from_xi2(info: &xi2::XITouchClassInfo) -> Self {
        let &xi2::XITouchClassInfo {
            _type: _, sourceid: _, mode, num_touches,
        } = info;
        assert!(num_touches >= 0);

        Self {
            num_touches: num_touches as _,
            mode: XI2TouchMode::from_xi2(mode).unwrap()
        }
    }
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct XI2KeyClassInfo {
    // Basically a Vec<Keycode>, but we don't care, do we? There's nothing to do with it; these
    // aren't even Keysyms.
}
impl XI2KeyClassInfo {
    pub fn from_xi2(info: &xi2::XIKeyClassInfo) -> Self {
        let &xi2::XIKeyClassInfo {
            _type: _, sourceid: _, num_keycodes: _, keycodes: _,
        } = info;
        Self {}
    }
}


#[repr(i32)]
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum XI2DeviceRole {
    MasterPointer  = xi2::XIMasterPointer,
    MasterKeyboard = xi2::XIMasterKeyboard,
    SlavePointer   = xi2::XISlavePointer,
    SlaveKeyboard  = xi2::XISlaveKeyboard,
    FloatingSlave  = xi2::XIFloatingSlave,
}

impl XI2DeviceRole {
    pub fn try_from_xi2_use(use_: c_int) -> Option<Self> {
        match use_ { 
            xi2::XIMasterPointer  => Some(XI2DeviceRole::MasterPointer),
            xi2::XIMasterKeyboard => Some(XI2DeviceRole::MasterKeyboard),
            xi2::XISlavePointer   => Some(XI2DeviceRole::SlavePointer),
            xi2::XISlaveKeyboard  => Some(XI2DeviceRole::SlaveKeyboard),
            xi2::XIFloatingSlave  => Some(XI2DeviceRole::FloatingSlave),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum XI2DeviceAnyClassInfo {
    Key(xi2::XIKeyClassInfo),
    Touch(xi2::XITouchClassInfo),
    Button(xi2::XIButtonClassInfo),
    Scroll(xi2::XIScrollClassInfo),
    Valuator(xi2::XIValuatorClassInfo),
}

impl XI2DeviceAnyClassInfo {
    pub unsafe fn try_from_anyclassinfo(info: *const xi2::XIAnyClassInfo) -> Option<Self> {
        assert!(!info.is_null());
        match (&*info)._type {
            xi2::XIKeyClass      => Some(XI2DeviceAnyClassInfo::Key(*(info as *const _ as *const xi2::XIKeyClassInfo))),
            xi2::XITouchClass    => Some(XI2DeviceAnyClassInfo::Touch(*(info as *const _ as *const xi2::XITouchClassInfo))),
            xi2::XIButtonClass   => Some(XI2DeviceAnyClassInfo::Button(*(info as *const _ as *const xi2::XIButtonClassInfo))),
            xi2::XIScrollClass   => Some(XI2DeviceAnyClassInfo::Scroll(*(info as *const _ as *const xi2::XIScrollClassInfo))),
            xi2::XIValuatorClass => Some(XI2DeviceAnyClassInfo::Valuator(*(info as *const _ as *const xi2::XIValuatorClassInfo))),
            _ => None,
        }
    }
}

pub unsafe fn xi2_query_single_device_info(x_display: *mut x::Display, deviceid: c_int, atoms: &PreloadedAtoms) -> Result<XI2DeviceInfo> {
    let info = super::device::xi2_query_device_info(x_display, deviceid, atoms)?.into_iter().next().unwrap();
    assert_eq!(info.0, deviceid);
    Ok(info.1)
}

pub unsafe fn xi2_query_device_info(x_display: *mut x::Display, querydeviceid: c_int, atoms: &PreloadedAtoms) -> Result<HashMap<c_int, XI2DeviceInfo>> {
    let mut ndevices = 0;
    let infos = xlib_error::sync_catch(x_display, || xi2::XIQueryDevice(x_display, querydeviceid, &mut ndevices))?;
    if ndevices <= 0 || infos.is_null() {
        return failed("XIQueryDevice() returned zero device infos");
    }
    let infos = slice::from_raw_parts_mut(infos, ndevices as _);
    let mut out_infos = HashMap::with_capacity(ndevices as _);

    for info in infos.iter() {
        let &xi2::XIDeviceInfo {
            deviceid, name, _use, attachment, enabled, num_classes, classes,
        } = info;

        let classes = slice::from_raw_parts(classes, num_classes as _);
        assert!(!name.is_null());

        let mut devinfo = XI2DeviceInfo {
            name: ::std::ffi::CStr::from_ptr(name).to_string_lossy().into(),
            role: XI2DeviceRole::try_from_xi2_use(_use),
            attachment,
            is_enabled: enabled != 0,
            key_class: None,
            button_class: None,
            touch_classes: vec![],
            valuator_classes: HashMap::new(),
            scroll_classes: HashMap::new(),
        };
        devinfo.replace_classes(classes.iter().filter_map(|x| XI2DeviceAnyClassInfo::try_from_anyclassinfo(*x)), atoms);
        out_infos.insert(deviceid, devinfo);
    }

    xi2::XIFreeDeviceInfo(infos.as_mut_ptr());
    Ok(out_infos)
}

pub unsafe fn xi2_list_device_properties(x_display: *mut x::Display, deviceid: c_int) -> Result<Vec<x::Atom>> {
    let mut nprops = 0;
    let props = xlib_error::sync_catch(x_display, || xi2::XIListProperties(x_display, deviceid, &mut nprops))?;
    let out = slice::from_raw_parts(props, nprops as _).to_vec();
    x::XFree(props as *mut _ as _);
    Ok(out)
}

pub unsafe fn xi2_get_device_property(x_display: *mut x::Display, deviceid: c_int, prop: x::Atom) -> Result<Option<XI2DeviceProperty>> {
    // format: 8, 16, 32
    // length: 32-bit multiples
    // offset: 32-bit quantities
    let offset = 0;
    let length = 1024; // Probably fine... That would be one heck of a property value otherwise, because this would be 4096 bytes.
    let delete_property = x::False;
    let type_hint = x::AnyPropertyType;

    let mut data = ptr::null_mut();
    let mut format = 0;
    let mut actual_type = 0;
    let mut nb_items = 0;
    let mut nb_bytes_remaining = 0;

    let status = xlib_error::sync_catch(x_display, || xi2::XIGetProperty(
        x_display, deviceid, prop,
        offset, length,
        delete_property,
        type_hint as _, &mut actual_type,
        &mut format,
        &mut nb_items, &mut nb_bytes_remaining, &mut data
    ))?;
    if status != x::Success as _ {
        return failed(format!("XIGetProperty() returned {}", status));
    }
    if actual_type == 0 { // Property doesn't exist for this device
        assert_eq!(format, 0);
        assert_eq!(nb_bytes_remaining, 0);
        assert_eq!(nb_items, 0);
        return Ok(None);
    }
    assert!(format == 8 || format == 16 || format == 32);
    let out = XI2DeviceProperty {
        format: format as _,
        type_: actual_type,
        data: slice::from_raw_parts(data, (nb_items * format as u64 / 8) as _).to_vec(),
    };
    x::XFree(data as *mut _ as _);
    Ok(Some(out))
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct XI2DeviceProperty {
    data: Vec<u8>,
    format: u8,
    type_: x::Atom,
}

impl XI2DeviceProperty {
    pub fn type_atom(&self) -> Option<x::Atom> {
        match self.type_ {
            0 => None,
            a => Some(a),
        }
    }
    pub fn item_bits(&self) -> usize {
        self.format as _
    }
    pub fn as_slice<T>(&self) -> Option<&[T]> {
        let sz = mem::size_of::<T>();
        if self.item_bits() != 8 * sz {
            return None;
        }
        Some(unsafe {
            slice::from_raw_parts(self.data.as_ptr() as *const T, self.data.len() / sz)
        })
    }
}

#[derive(Debug)]
pub struct XI2DeviceCache {
    pub info: XI2DeviceInfo,
    pub props: HashMap<x::Atom, XI2DeviceProperty>,
}

pub unsafe fn refresh_xi2_device_cache(x_display: *mut x::Display, deviceid: c_int, atoms: &PreloadedAtoms) -> Result<XI2DeviceCache> {
    let info = xi2_query_single_device_info(x_display, deviceid, atoms)?;
    let props = atoms.interesting_xi2_props()
        .iter()
        .filter_map(|k| xi2_get_device_property(x_display, deviceid, *k).ok().map(|v| (k, v)))
        .filter_map(|(k, v)| v.map(|v| (*k, v)))
        .collect();
    Ok(XI2DeviceCache { info, props })
}

