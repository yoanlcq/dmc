use error::{Result, unsupported_unexplained};
use hint::Hint;

pub fn set_hint(hint: Hint) -> Result<()> {
    match hint {
        Hint::XlibDefaultErrorHandlers(_) => unsupported_unexplained(),
        Hint::XlibXInitThreads => unsupported_unexplained(),
    }
}
