macro_rules! glx_ext {
    (($($name:ident)+) ($($func:ident)+)) => {
        #[allow(non_snake_case)]
        #[derive(Debug, Copy, Clone, Default, Hash, PartialEq, Eq)]
        pub struct GlxExt {
            $(pub $name: bool,)+
            $(pub $func: Option<types::$func>,)+
        }
        impl GlxExt {
            #[allow(non_snake_case)]
            pub fn parse(gpa: types::glXGetProcAddress, s: &CStr) -> Self {
                $(let mut $name = false;)+
                let s = s.to_string_lossy();
                for name in s.split_whitespace() {
                    match name {
                        $(stringify!($name) => {
                            $name = true;
                            info!("Found GLX extension {}", stringify!($name));
                        },)+
                        _ => {}
                    };
                }
                let mut out = Self { $($name,)+ $($func: None,)+ };

                // Load functions
                unsafe { $(
                    let cstring = CString::new(stringify!($func)).unwrap_or_default();
                    let name = cstring.to_bytes_with_nul();
                    let fptr = gpa(name.as_ptr() as *mut _);
                    out.$func = match fptr {
                        None => {
                            warn!("Couldn't load `{}`", stringify!($func));
                            None
                        },
                        Some(f) => {
                            info!("Loaded `{}`", stringify!($func));
                            Some(mem::transmute(f))
                        },
                    };
                )+ }

                out
            }
        }
    }
}


glx_ext!((
    GLX_ARB_multisample
    GLX_EXT_swap_control
    GLX_EXT_swap_control_tear
    GLX_MESA_swap_control
    GLX_SGI_swap_control
    GLX_SGI_video_sync
    GLX_OML_swap_method
    GLX_OML_sync_control
    GLX_ARB_create_context
    GLX_ARB_create_context_profile
    GLX_ARB_create_context_robustness
    GLX_EXT_create_context_es_profile
    GLX_EXT_create_context_es2_profile
    )(
    glXSwapIntervalEXT
    glXSwapIntervalMESA
    glXGetSwapIntervalMESA
    glXSwapIntervalSGI
    glXCreateContextAttribsARB
));


