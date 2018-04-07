use os::OsCursor;
use std::time::Duration;
use super::{Vec2, Rgba, Extent2};

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
    pub size: Extent2<u32>,
    pub rgba_data: Vec<Rgba<u8>>,
    pub and_mask: Vec<bool>,
    pub xor_mask: Vec<bool>,
}

