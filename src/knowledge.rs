
option_alternative!{
    /// A value which is either known or unknown.
    /// 
    /// APIs should use this instead
    /// of Option<T> when the conveyed meaning is more appropriate.
    #[allow(missing_docs)]
    pub enum Knowledge<T> {
        Known(T),
        Unknown,
    }
    is_known
    is_known_some
    is_known_none
    is_unknown
}

