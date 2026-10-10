use crate::Error;
use tokio_postgres::types::{FromSql, Type};

/// Retained metadata bounds. Provider SQL must also bound projected native values.
/// These limits do not bound the SDK's allocation of one incoming protocol frame.
#[derive(Clone, Copy, Debug)]
pub struct MetadataLimits {
    pub(crate) rows: usize,
    pub(crate) columns: usize,
    pub(crate) bytes: usize,
}
impl MetadataLimits {
    /// Admit1..=32 rows,1..=16 columns and1..=64KiB decoded bytes in the complete result.
    pub fn new(rows: usize, columns: usize, bytes: usize) -> Result<Self, Error> {
        if !(1..=32).contains(&rows)
            || !(1..=16).contains(&columns)
            || !(1..=65536).contains(&bytes)
        {
            return Err(Error::Invalid);
        }
        Ok(Self {
            rows,
            columns,
            bytes,
        })
    }
}
pub(crate) struct FieldBytes<'a>(pub(crate) &'a [u8]);
impl<'a> FromSql<'a> for FieldBytes<'a> {
    fn from_sql(_: &Type, raw: &'a [u8]) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
        Ok(Self(raw))
    }
    fn accepts(_: &Type) -> bool {
        true
    }
}
