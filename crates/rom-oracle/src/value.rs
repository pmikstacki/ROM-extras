use rom_sql_core::OwnerError;
/// Owned finite native binds crossing the private worker. Empty RAW/text follows Oracle NULL semantics.
#[derive(Clone)]
pub enum Value {
    /// Nullable signed integer bound as native NUMBER.
    Integer(Option<i64>),
    /// Nullable opaque binary bytes bound as native RAW.
    Raw(Option<Vec<u8>>),
    /// Nullable UTF-8 text bound as native NVARCHAR2 by the SDK.
    Text(Option<String>),
    /// Nullable finite double bound as native BINARY_DOUBLE.
    Double(Option<f64>),
}
impl Value {
    pub(crate) fn native(&self) -> &dyn oracle::sql_type::ToSql {
        match self {
            Self::Integer(v) => v,
            Self::Raw(v) => v,
            Self::Text(v) => v,
            Self::Double(v) => v,
        }
    }
    pub(crate) fn size(&self) -> Result<usize, OwnerError> {
        let size = match self {
            Self::Integer(_) => 8,
            Self::Double(v) => {
                if v.is_some_and(|x| !x.is_finite()) {
                    return Err(OwnerError::Invalid);
                }
                8
            }
            Self::Raw(v) => v.as_ref().map_or(0, Vec::len),
            Self::Text(v) => v.as_ref().map_or(0, String::len),
        };
        if size > 32767 {
            Err(OwnerError::Invalid)
        } else {
            Ok(size)
        }
    }
}
impl std::fmt::Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Value { redacted }")
    }
}
