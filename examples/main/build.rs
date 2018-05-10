extern crate rustc_version;
use rustc_version::{version, version_meta, Channel};

fn main() {
    assert!(version().unwrap().major >= 1);

    match version_meta().unwrap().channel {
        Channel::Stable => {
            println!("cargo:rustc-cfg=stable");
        }
        Channel::Beta => {
            println!("cargo:rustc-cfg=beta");
        }
        Channel::Nightly => {
            println!("cargo:rustc-cfg=nightly");
        }
        Channel::Dev => {
            println!("cargo:rustc-cfg=dev");
        }
    }

    #[cfg(any(target_os="linux", target_os="freebsd", target_os="dragonfly", target_os="openbsd", target_os="netbsd"))]
    println!("cargo:rustc-cfg=x11");
}

