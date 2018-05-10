use std::os::raw::{c_ulong, c_long, c_ushort, c_short, c_char, c_uchar, c_int};
use std::ops::Range;
use std::ptr;
use super::x11::xlib as x;
use super::xlib_error;
use error::{Result, failed};

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
#[repr(u64)]
pub enum PropType {
    Any = x::AnyPropertyType as _,
    Cardinal = x::XA_CARDINAL,
    Atom = x::XA_ATOM,
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
#[repr(i32)]
pub enum PropMode {
    Replace = x::PropModeReplace,
    Append = x::PropModeAppend,
    #[allow(dead_code)]
    Prepend = x::PropModePrepend,
}

#[derive(Debug, Default, Clone, Hash, PartialEq, Eq)]
pub struct PropData<T> {
    pub data: Vec<T>,
    pub bytes_remaining_to_be_read: usize,
}

pub trait PropElement {
    const SERVER_BITS: usize;
}

impl PropElement for c_ulong  { const SERVER_BITS: usize = 32; }
impl PropElement for c_long   { const SERVER_BITS: usize = 32; }
impl PropElement for c_ushort { const SERVER_BITS: usize = 16; }
impl PropElement for c_short  { const SERVER_BITS: usize = 16; }
impl PropElement for c_uchar  { const SERVER_BITS: usize =  8; }
impl PropElement for c_char   { const SERVER_BITS: usize =  8; }


pub fn set<T: PropElement>(x_display: *mut x::Display, x_window: x::Window, prop: x::Atom, prop_type: PropType, mode: PropMode, data: &[T]) -> Result<()> {
    unsafe {
        xlib_error::sync_catch(x_display, || {
            x::XChangeProperty(x_display, x_window, prop, prop_type as _, T::SERVER_BITS as _, mode as _, data.as_ptr() as *const _ as *mut _, data.len() as _);
        })
    }
}

// `long_offset` and `long_length` are expressed in multiples of server-side 32-bit elements.
pub fn get<T: PropElement>(x_display: *mut x::Display, x_window: x::Window, prop: x::Atom, req_type: PropType, long_range: Range<usize>) -> Result<PropData<T>> {
    assert!(long_range.start <= long_range.end);
    let long_offset = long_range.start;
    let long_length = long_range.end - long_range.start;
    let delete: x::Bool = x::False;
    let mut actual_format_return: c_int = 0;
    let mut actual_type_return: x::Atom = 0;
    let mut nitems_return: c_ulong = 0;
    let mut bytes_remaining_to_be_read: c_ulong = 0;

    let mut data_ptr: *mut c_uchar = ptr::null_mut();
    let status = unsafe { xlib_error::sync_catch(x_display, || {
        x::XGetWindowProperty(
            x_display, x_window, prop, long_offset as _, long_length as _, delete,
            req_type as _, &mut actual_type_return,
            &mut actual_format_return, &mut nitems_return,
            &mut bytes_remaining_to_be_read, &mut data_ptr
        )
    })}?;
    if status != x::Success as _ {
        return failed(format!("XGetWindowProperty() returned {}", status));
    }
    let mut data = Vec::with_capacity(nitems_return as usize);
    unsafe {
        ptr::copy_nonoverlapping(data_ptr as *const _  as *const T, data.as_mut_ptr(), nitems_return as usize);
        x::XFree(data_ptr as _);
        data.set_len(nitems_return as usize);
    }
    if actual_type_return == 0 || actual_format_return == 0 {
        return failed("Property doesn't exist for this window");
    }
    if actual_format_return != T::SERVER_BITS as _ {
        return failed(format!("The actual format for this property is {} server-side bits", actual_format_return));
    }
    Ok(PropData {
        data, bytes_remaining_to_be_read: bytes_remaining_to_be_read as _,
    })
}

