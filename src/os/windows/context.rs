use error::Result;

#[derive(Debug)]
pub struct OsContext;

impl OsContext {
    pub fn new() -> Result<Self> {
        Ok(OsContext)
    }
    pub fn untrap_mouse(&self) -> Result<()> {
        unimplemented!()
    }
}

