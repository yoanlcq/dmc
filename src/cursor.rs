use os::OsCursor;
use std::time::Duration;
use super::{Vec2, Rgba};

#[derive(Debug)]
pub struct Cursor(OsCursor);

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub enum SystemCursor {
    Arrow,
    Hand,
    Ibeam,
    Wait,
    Crosshair,
    WaitArrow,
    SizeNorthWestToSouthEast,
    SizeNorthEastToSouthWest,
    SizeVertical,
    SizeHorizontal,
    SizeAll,
    Deny,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct Image<T>(T); // FIXME

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct CursorFrame {
    pub duration: Duration,
    pub hotspot: Vec2<u32>,
    pub image: Image<Rgba<u8>>,
    pub and_mask: Image<bool>,
    pub xor_mask: Image<bool>,
}

