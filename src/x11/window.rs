extern crate libc;

use std::ptr;
use std::rc::Rc;
use std::os::raw::c_void;
use super::x11::xlib as x;
use super::{X11Context, X11SharedContext};
use window::{WindowSettings, WindowMode};
use error::{Result, failed};

#[derive(Debug)]
pub struct X11Window {
    pub context: Rc<X11SharedContext>,
    pub x_window: x::Window,
    pub xic: Option<x::XIC>,
}

impl Drop for X11Window {
    fn drop(&mut self) {
        let &mut Self {
            ref mut context, x_window, xic,
        } = self;
        let x_display = context.x_display;
        unsafe {
            if let Some(xic) = xic {
                x::XDestroyIC(xic);
            }
            x::XDestroyWindow(x_display, x_window);
        }
    }
}


impl X11Context {
    pub fn create_window(&self, window_settings: &WindowSettings) -> Result<X11Window> {
        let context = Rc::clone(&self.0);
        let x_display = context.x_display;

        // Creating the X Window.
        // There's a lot of arguments to set up!

        let parent = unsafe {
            x::XDefaultRootWindow(x_display)
        };

        let &WindowSettings {
            mode, resizable, fully_opaque, ref opengl, allow_high_dpi
        } = window_settings;

        let (w, h, maximized, fullscreen) = match mode {
            WindowMode::FixedSize(w, h) => (w, h, false, false),
            WindowMode::Maximized => unimplemented!{},
            WindowMode::FullScreen => unimplemented!{},
        };
        let (x, y) = (0, 0);

        let (visual, depth, colormap) = match *opengl {
            Some(ref pixel_format) => {
                unimplemented!{"We need to load the GLX extension"}
                /*
                if self.glx.is_none() {
                    return failed("Cannot create OpenGL-capable window without GLX");
                }
                let vi = unsafe { *pixel_format.0.visual_info };
                let colormap = unsafe {
                    x::XCreateColormap(x_display, parent, vi.visual, x::AllocNone)
                };
                (vi.visual, vi.depth, colormap)
                */
            },
            None => {
                let depth = x::CopyFromParent;
                let visual = unsafe {
                    x::XDefaultVisual(x_display, 0 /* screen_num */)
                };
                let colormap = unsafe {
                    x::XCreateColormap(x_display, parent, visual, x::AllocNone)
                };
                (visual, depth, colormap)
            },
        };

        let border_thickness = 0;
        let class = x::InputOutput;
        let valuemask = x::CWBorderPixel | x::CWColormap | x::CWEventMask;
        let mut swa = x::XSetWindowAttributes {
            colormap,
            event_mask:
                x::ButtonReleaseMask      | x::EnterWindowMask | x::ButtonPressMask |
                x::LeaveWindowMask        | x::PointerMotionMask | 
                x::Button1MotionMask      |
                x::Button2MotionMask      | x::Button3MotionMask |
                x::Button4MotionMask      | x::Button5MotionMask |
                x::ButtonMotionMask       | x::KeymapStateMask |
                x::ExposureMask           | x::VisibilityChangeMask | 
                x::StructureNotifyMask    | /* ResizeRedirectMask | */
                x::SubstructureNotifyMask | x::SubstructureRedirectMask |
                x::FocusChangeMask        | x::PropertyChangeMask |
                x::ColormapChangeMask     | x::OwnerGrabButtonMask,
            background_pixmap    : 0,  
            background_pixel     : 0,  
            border_pixmap        : 0,  
            border_pixel         : 0,  
            bit_gravity          : 0,  
            win_gravity          : 0,  
            backing_store        : 0,  
            backing_planes       : 0,  
            backing_pixel        : 0,  
            save_under           : 0,  
            do_not_propagate_mask: 0,  
            override_redirect    : 0,  
            cursor               : 0,  
        };

        let x_window = unsafe {
            x::XCreateWindow(
                x_display, parent, x, y, w, h,
                border_thickness, depth, class as _, visual, valuemask, &mut swa
            )
        };
        
        // FIXME: Install an X error handler!

        if x_window == 0 {
            return failed("XCreateWindow() returned 0");
        }

        // The next step 

        unsafe {
            let mut protocols = [ 
                self.atoms.WM_DELETE_WINDOW,
                self.atoms._NET_WM_PING,
                self.atoms.WM_TAKE_FOCUS,
            ];
            x::XSetWMProtocols(
                x_display, x_window, protocols.as_mut_ptr(), protocols.len() as _
            );

            let pid = libc::getpid();
            if pid > 0 {
                x::XChangeProperty(
                    x_display, x_window, self.atoms._NET_WM_PID, 
                    x::XA_CARDINAL, 32, x::PropModeReplace,
                    &pid as *const _ as *const _, 
                    1
                );
            }
            /*
            x::XChangeProperty(
                x_display, x_window, self.atoms.XdndAware, 
                x::XA_ATOM, 32, x::PropModeReplace,
                &xdnd_version as *const _ as *const _, 
                1
            );
            */
        }


        // Getting an X Input Context for this window
        let xic = if let Some(xim) = self.xim {
            let xic = unsafe {
                x::XCreateIC(xim, 
                    x::XNClientWindow, x_window,
                    x::XNFocusWindow, x_window,
                    x::XNInputStyle, x::XIMPreeditNothing | x::XIMStatusNothing,
                    ptr::null_mut() as *mut c_void,
                )
            };
            if xic.is_null() {
                None
            } else {
                Some(xic)
            }
        } else {
            None
        };

        Ok(X11Window { context, x_window, xic, })
    }
}

