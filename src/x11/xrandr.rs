// XRRGetScreenSizeRange(0x18f3010, 705, 0x60e3bc, 0x60e3b4)             
// XRRGetScreenResources(0x18f3010, 705, 0, 0)                           
//
// XRRGetCrtcInfo(0x18f3010, 0x19031c0, 63, 0x1902fc0)                   
// XRRGetPanning(0x18f3010, 0x19031c0, 63, 0)                            
// XRRGetCrtcTransform(0x18f3010, 63, 0x7ffd59ead930, 0x19031c0)         
// XFree(0x19026b0, 0x1902730, 0, 0x7f453fa85b00)                        
//
// XRRGetOutputInfo(0x18f3010, 0x19031c0, 66, 0)                         
// XRRGetCrtcGammaSize(0x18f3010, 63, 0x1903cc0, 1)                      
// XRRGetCrtcGamma(0x18f3010, 63, 0, 0)                                  
// XRRFreeGamma(0x19045f0, 1022, 0x7f453fb6ebe0, 1200)                   
//
// XRRGetOutputPrimary(0x18f3010, 705, 0, 0x7f453fa85b00)                
// XRRGetOutputInfo(0x18f3010, 0x19031c0, 67, 8)                         

use std::os::raw::c_int;
use error::{Result, failed};
use super::x11::xlib as x;

#[derive(Debug, Default, Clone, Hash, PartialEq, Eq)]
pub struct XRR {
    pub event_base: c_int,
    pub error_base: c_int,
    pub major_version: c_int,
    pub minor_version: c_int,
}

impl X11SharedContext {
    fn query_xrr(&self) -> Result<XRR> {
        let x_display = self.x_display;
        let mut xrr = XRR::default();
        let success = unsafe {
            x::XRRQueryExtension(x_display, &mut xrr.event_base, &mut xrr.error_base)
        };
        if success == x::False as _ {
            return failed("XRRQueryExtension() returned False");
        }
        let status = unsafe {
            x::XRRQueryVersion(x_display, &mut xrr.major_version, &mut xrr.minor_version)
        };
        if status != x::Success as _ {
            return failed(format!("XRRQueryExtension() returned {}", status));
        }
    }
}
