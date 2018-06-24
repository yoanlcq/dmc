#[cfg(x11)]
extern crate x11;

use std::time::Duration;
use std::collections::VecDeque;
use dmc::{self, Context, Window, WindowSettings, WindowTypeHint, Event, Extent2, Vec2, Rect};

#[derive(Debug, Default)]
pub struct App {
    context: Option<Context>,
    main_window: Option<Window>,
}

macro_rules! tests {
    ($($(#[$attr:meta])* $Variant:ident => $run:expr,)+) => {
        pub mod test {
            use super::*;

            #[derive(Debug, Clone, Hash, PartialEq, Eq)]
            pub enum WhatNow {
                RunNextTest,
                WaitForApproval,
                #[allow(dead_code)]
                Sleep(Duration),
            }

            pub fn wait_for_approval() -> self::Result {
                Ok(WhatNow::WaitForApproval)
            }
            pub fn run_next_test() -> self::Result {
                Ok(WhatNow::RunNextTest)
            }
            #[allow(dead_code)]
            pub fn sleep(d: Duration) -> self::Result {
                Ok(WhatNow::Sleep(d))
            }

            pub type Result = dmc::error::Result<WhatNow>;

            #[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
            pub enum Test {
                $($(#[$attr])* $Variant,)+
            }

            pub static ALL_TESTS: &[Test] = &[$($(#[$attr])* Test::$Variant,)+];

            impl App {
                pub fn run_test(&mut self, test: Test) -> self::Result {
                    match test {
                        $($(#[$attr])* Test::$Variant => ($run)(self),)+
                    }
                }
            }
        }
    };
}

#[cfg(feature="headless")]
tests!{
    // May change in the future
    DoNothingBecauseWeAreHeadless => App::do_nothing_because_we_are_headless,
}

#[cfg(not(feature="headless"))]
tests!{
    #[cfg(x11)]
    CreateContextWithX11DisplayNameNone => App::test_create_context_with_x11_display_name_none,
    #[cfg(x11)]
    CreateContextWithX11DisplayNameSome => App::test_create_context_with_x11_display_name_some,
    #[cfg(x11)]
    CreateContextFromXlibDisplay => App::test_create_context_from_xlib_display,
    InitContext => App::init_context,
    InitWindow => App::init_window,
    WindowSetTypeHint => App::window_set_type_hint,
    WindowSetTitle => App::window_set_title,
    WindowRetrieveTitle => App::window_retrieve_title,
    WindowRaise => App::window_raise,
    WindowShow => App::window_show,
    WindowHide => App::window_hide,
    WindowToggleVisibility1 => App::window_toggle_visibility,
    WindowToggleVisibility2 => App::window_toggle_visibility,
    WindowToggleVisibility3 => App::window_toggle_visibility,
    WindowSetHalfOpacity => App::window_set_half_opacity,
    WindowSetZeroOpacity => App::window_set_zero_opacity,
    WindowResetOpacity => App::window_reset_opacity,
    WindowSetMinSize => App::window_set_min_size,
    WindowSetMaxSize => App::window_set_max_size,
    WindowResetMinMaxSize => App::window_reset_min_max_size,
    WindowMaximize => App::window_maximize,
    WindowUnmaximize => App::window_unmaximize,
    WindowToggleMaximize => App::window_toggle_maximize,
    WindowToggleMaximize2 => App::window_toggle_maximize,
    WindowMinimize => App::window_minimize,
    WindowUnminimize => App::window_unminimize,
    WindowToggleMinimize1 => App::window_toggle_minimize,
    WindowToggleMinimize2 => App::window_toggle_minimize,
    WindowEnterFullscreen => App::window_enter_fullscreen,
    WindowSetHalfOpacityWhileFullscreen => App::window_set_half_opacity,
    WindowSetZeroOpacityWhileFullscreen => App::window_set_zero_opacity,
    WindowResetOpacityWhileFullscreen => App::window_reset_opacity,
    WindowLeaveFullscreen => App::window_leave_fullscreen,
    WindowToggleFullscreen1 => App::window_toggle_fullscreen,
    WindowToggleFullscreen2 => App::window_toggle_fullscreen,
    WindowSetPosition => App::window_set_position,
    WindowSetSize => App::window_set_size,
    WindowSetPositionAndSize => App::window_set_position_and_size,
    WindowDemandAttention => App::window_demand_attention,
    WindowDemandUrgentAttention => App::window_demand_urgent_attention,
}


use self::test::{run_next_test, wait_for_approval};

fn failed_error(reason: &'static str) -> dmc::Error {
    dmc::Error {
        kind: dmc::ErrorKind::Failed,
        reason: Some(reason.into()),
    }
}

fn failed(reason: &'static str) -> test::Result {
    Err(failed_error(reason))
}


impl App {
    #[cfg_attr(not(feature="headless"), allow(dead_code))]
    fn do_nothing_because_we_are_headless(&mut self) -> test::Result {
        run_next_test()
    }

    pub fn init_simple_app(&mut self) -> test::Result {
        self.init_context()?;
        self.init_window()?;
        self.window_set_type_hint()?;
        self.window_set_title()?;
        self.window_raise()?;
        self.window_show()?;
        run_next_test()
    }

    #[cfg(x11)]
    fn test_create_context_with_x11_display_name_none(&mut self) -> test::Result {
        Context::with_x11_display_name(None)?;
        run_next_test()
    }
    #[cfg(x11)]
    fn test_create_context_with_x11_display_name_some(&mut self) -> test::Result {
        let name = ::std::ffi::CStr::from_bytes_with_nul(b":0.0\0").unwrap();
        Context::with_x11_display_name(Some(name))?;
        run_next_test()
    }
    #[cfg(x11)]
    fn test_create_context_from_xlib_display(&mut self) -> test::Result {
        // NOTE: Don't do XCloseDisplay(dpy), the context takes ownership of it!
        unsafe {
            Context::from_xlib_display(x11::xlib::XOpenDisplay(::std::ptr::null()))?;
        }
        run_next_test()
    }

    const MAIN_WINDOW_NAME: &'static str = "😎😎 → DMC demo ✨😎😎";

    fn window_error() -> dmc::Error {
        failed_error("Window was not created")
    }
    #[allow(dead_code)]
    pub fn main_window_mut(&mut self) -> dmc::error::Result<&mut Window> {
        self.main_window.as_mut().map(Ok).unwrap_or(Err(Self::window_error()))
    }
    pub fn main_window(&self) -> dmc::error::Result<&Window> {
        self.main_window.as_ref().map(Ok).unwrap_or(Err(Self::window_error()))
    }

    fn context_error() -> dmc::Error {
        failed_error("Context was not created")
    }
    #[allow(dead_code)]
    pub fn context_mut(&mut self) -> dmc::error::Result<&mut Context> {
        self.context.as_mut().map(Ok).unwrap_or(Err(Self::context_error()))
    }
    pub fn context(&self) -> dmc::error::Result<&Context> {
        self.context.as_ref().map(Ok).unwrap_or(Err(Self::context_error()))
    }


    fn init_context(&mut self) -> test::Result {
        self.context = Some(Context::new()?);
        #[cfg(x11)]
        self.context()?.xlib_xsynchronize(false); // On purpose
        run_next_test()
    }
    fn init_window(&mut self) -> test::Result {
        self.main_window = Some(self.context()?.create_window(&WindowSettings {
            opengl: None,
            high_dpi: true,
        })?);
        self.main_window()?.clear()?;
        run_next_test()
    }
    fn window_set_type_hint(&mut self) -> test::Result {
        self.main_window()?.set_type_hint(&WindowTypeHint::default())?;
        self.main_window()?.clear()?;
        run_next_test()
    }
    fn window_set_title(&mut self) -> test::Result {
        self.main_window()?.set_title(Self::MAIN_WINDOW_NAME)?;
        run_next_test()
    }
    fn window_retrieve_title(&mut self) -> test::Result {
        let title = self.main_window()?.title()?;
        match title.as_str() {
            Self::MAIN_WINDOW_NAME => run_next_test(),
            _ => failed("Window title is not the same as the one that was set"),
        }
    }
    fn window_raise(&mut self) -> test::Result {
        self.main_window()?.raise()?;
        self.main_window()?.clear()?;
        wait_for_approval()
    }
    fn window_show(&mut self) -> test::Result {
        self.main_window()?.show()?;
        self.main_window()?.clear()?;
        wait_for_approval()
    }
    fn window_hide(&mut self) -> test::Result {
        self.main_window()?.hide()?;
        wait_for_approval()
    }
    fn window_toggle_visibility(&mut self) -> test::Result {
        self.main_window()?.toggle_visibility()?;
        self.main_window()?.clear()?;
        wait_for_approval()
    }
    fn window_set_half_opacity(&mut self) -> test::Result {
        self.main_window()?.set_opacity(0.5)?;
        wait_for_approval()
    }
    fn window_set_zero_opacity(&mut self) -> test::Result {
        self.main_window()?.set_opacity(0.)?;
        wait_for_approval()
    }
    fn window_reset_opacity(&mut self) -> test::Result {
        self.main_window()?.set_opacity(1.)?;
        run_next_test()
    }
    fn window_set_min_size(&mut self) -> test::Result {
        self.main_window()?.clear()?;
        self.main_window()?.set_min_size(Extent2::new(500, 500))?;
        self.main_window()?.clear()?;
        wait_for_approval()
    }
    fn window_set_max_size(&mut self) -> test::Result {
        self.main_window()?.clear()?;
        self.main_window()?.set_max_size(Extent2::new(400, 400))?;
        self.main_window()?.clear()?;
        wait_for_approval()
    }
    fn window_reset_min_max_size(&mut self) -> test::Result {
        self.main_window()?.set_min_size(Extent2::new(0, 0))?;
        self.main_window()?.set_max_size(Extent2::new(99999999, 99999999))?;
        self.main_window()?.clear()?;
        run_next_test()
    }
    fn window_maximize(&mut self) -> test::Result {
        self.main_window()?.maximize()?;
        self.main_window()?.clear()?;
        wait_for_approval()
    }
    fn window_unmaximize(&mut self) -> test::Result {
        self.main_window()?.unmaximize()?;
        wait_for_approval()
    }
    fn window_toggle_maximize(&mut self) -> test::Result {
        self.main_window()?.toggle_maximize()?;
        self.main_window()?.clear()?;
        wait_for_approval()
    }
    fn window_minimize(&mut self) -> test::Result {
        self.main_window()?.minimize()?;
        wait_for_approval()
    }
    fn window_unminimize(&mut self) -> test::Result {
        self.main_window()?.unminimize()?;
        self.main_window()?.clear()?;
        wait_for_approval()
    }
    fn window_toggle_minimize(&mut self) -> test::Result {
        self.main_window()?.toggle_minimize()?;
        self.main_window()?.clear()?;
        wait_for_approval()
    }
    fn window_enter_fullscreen(&mut self) -> test::Result {
        self.main_window()?.enter_fullscreen()?;
        self.main_window()?.clear()?;
        wait_for_approval()
    }
    fn window_leave_fullscreen(&mut self) -> test::Result {
        self.main_window()?.leave_fullscreen()?;
        wait_for_approval()
    }
    fn window_toggle_fullscreen(&mut self) -> test::Result {
        self.main_window()?.toggle_fullscreen()?;
        self.main_window()?.clear()?;
        wait_for_approval()
    }
    fn window_demand_attention(&mut self) -> test::Result {
        self.main_window()?.demand_attention()?;
        wait_for_approval()
    }
    fn window_demand_urgent_attention(&mut self) -> test::Result {
        self.main_window()?.demand_urgent_attention()?;
        wait_for_approval()
    }
    fn window_set_position(&mut self) -> test::Result {
        self.main_window()?.set_position(Vec2::new(100, 100))?;
        wait_for_approval()
    }
    fn window_set_size(&mut self) -> test::Result {
        self.main_window()?.set_size(Extent2::new(600, 600))?;
        self.main_window()?.clear()?;
        wait_for_approval()
    }
    fn window_set_position_and_size(&mut self) -> test::Result {
        self.main_window()?.set_position_and_size(Rect { x: 200, y: 200, w: 200, h: 200, })?;
        self.main_window()?.clear()?;
        wait_for_approval()
    }
}

impl App {
    pub fn pump_events(&mut self) -> VecDeque<Event> {
        let mut pumped = VecDeque::new();
        if let Some(context) = self.context.as_ref() {
            for ev in context.events_poll_iter() {
                info!("----- {:?}", ev);
                pumped.push_back(ev);
            }
        }
        pumped
    }
}
