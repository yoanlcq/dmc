use os::OsCursor;
use std::time::Duration;
use image::Image;
use super::{Vec2, Rgba};

#[derive(Debug)]
pub struct Cursor(pub(crate) OsCursor);

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub enum SystemCursor {
    Arrow,
    Hand,
    Ibeam,
    Wait,
    Crosshair,
    WaitArrow,
    ResizeNWToSE,
    ResizeNEToSW,
    ResizeV,
    ResizeH,
    ResizeHV,
    Deny,
    Question,
    ReverseArrow,
    TopSide,
    BottomSide,
    LeftSide,
    RightSide,
    BottomLeftCorner,
    BottomRightCorner,
    TopLeftCorner,
    TopRightCorner,
    Pencil,
    Spraycan,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct CursorFrame {
    pub duration: Duration,
    pub data: CursorData,
}
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct CursorData {
    pub hotspot: Vec2<u32>,
    pub image: Image<Rgba<u8>>,
    pub and_mask: Image<bool>,
    pub xor_mask: Image<bool>,
}

