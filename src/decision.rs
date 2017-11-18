
option_alternative!{
    /// Type for values which may be either set manually or left for the
    /// implementation to decide.
    /// 
    /// The actual meaning of `Auto` is often
    /// "best setting", but this is not required as it depends too much
    /// on context.
    /// 
    /// APIs should use this instead
    /// of Option<T> when the conveyed meaning is more appropriate.
    /// 
    /// For a rationale, see [https://english.stackexchange.com/a/203664](https://english.stackexchange.com/a/203664)
    #[allow(missing_docs)]
    pub enum Decision<T> {
        Manual(T),
        Auto,
    }
    is_manual
    is_manual_some
    is_manual_none
    is_auto
}
