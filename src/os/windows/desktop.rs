use desktop::Desktop;
use error::Result;
use super::OsContext;

impl OsContext {
    pub fn desktops(&self) -> Result<Vec<Desktop>> {
        unimplemented!()
    }
    pub fn current_desktop(&self) -> Result<usize> {
        unimplemented!()
    }
}
