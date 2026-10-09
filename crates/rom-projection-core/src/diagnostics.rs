//! Constant-name Debug implementations for sensitive core values.
macro_rules! sanitized_debug {
    ($($type:ty),+ $(,)?) => {$(impl std::fmt::Debug for $type {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(stringify!($type))
        }
    })+};
}

pub(crate) use sanitized_debug;
