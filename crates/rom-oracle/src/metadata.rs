use crate::{ControlTable, worker::Failure};
use oracle::sql_type::OracleType;
use rom_sql_core::{OwnerError, OwnerState, validate_owner_row};
fn invalid() -> Failure {
    Failure::Rejected(OwnerError::Invalid)
}
pub(crate) fn read(
    c: &oracle::Connection,
    table: &ControlTable,
    lock: u64,
) -> Result<OwnerState, Failure> {
    // Describe original columns without rows before CASE can coerce types or fail preparation.
    // The host owns stable schema; this is not protection against concurrent administrative DDL.
    {
        let mut stmt = c
            .statement(&format!(
                "SELECT format_version,store_identity,generation,owner_token FROM {} WHERE 1=0",
                table.qualified
            ))
            .fetch_array_size(1)
            .prefetch_rows(0)
            .build()?;
        let rows = stmt.query(&[])?;
        let info = rows.column_info();
        if info.len() != 4
            || ![0, 2]
                .into_iter()
                .all(|i| matches!(info[i].oracle_type(), OracleType::Number(_, _)))
            || ![1, 3]
                .into_iter()
                .all(|i| matches!(info[i].oracle_type(), OracleType::Raw(_)))
        {
            return Err(invalid());
        }
    }
    let sql = format!(
        "SELECT format_version,CASE WHEN UTL_RAW.LENGTH(store_identity)=32 THEN store_identity ELSE CAST(NULL AS RAW(32)) END,generation,CASE WHEN UTL_RAW.LENGTH(owner_token)=32 THEN owner_token ELSE CAST(NULL AS RAW(32)) END,UTL_RAW.LENGTH(owner_token),CASE WHEN format_version=1 THEN 1 ELSE 0 END,CASE WHEN generation=TRUNC(generation) AND generation BETWEEN 0 AND 9223372036854775807 THEN 1 ELSE 0 END FROM {} WHERE singleton_key=:1 AND ROWNUM<=2 FOR UPDATE WAIT {lock}",
        table.qualified
    );
    let mut stmt = c
        .statement(&sql)
        .fetch_array_size(2)
        .prefetch_rows(0)
        .build()?;
    let mut rows = stmt.query(&[&1_i64])?;
    let info = rows.column_info();
    if info.len() != 7
        || ![0, 2, 4, 5, 6]
            .into_iter()
            .all(|i| matches!(info[i].oracle_type(), OracleType::Number(_, _)))
        || ![1, 3]
            .into_iter()
            .all(|i| matches!(info[i].oracle_type(), OracleType::Raw(_)))
    {
        return Err(invalid());
    }
    let row = match rows.next() {
        Some(r) => r?,
        None => return Err(Failure::Rejected(OwnerError::Uninitialized)),
    };
    if rows.next().transpose()?.is_some() {
        return Err(invalid());
    }
    // Check exact native NUMBER predicates before getters that can otherwise coerce or truncate.
    if row.get::<_, i64>(5).map_err(|_| invalid())? != 1 {
        return Err(Failure::Rejected(OwnerError::UnsupportedFormat));
    }
    if row.get::<_, i64>(6).map_err(|_| invalid())? != 1 {
        return Err(invalid());
    }
    let identity = row.get::<_, Option<Vec<u8>>>(1).map_err(|_| invalid())?;
    let token = row.get::<_, Option<Vec<u8>>>(3).map_err(|_| invalid())?;
    let generation = row.get::<_, i64>(2).map_err(|_| invalid())?;
    let length = row.get::<_, Option<i64>>(4).map_err(|_| invalid())?;
    validate_owner_row(
        1,
        &table.identity,
        identity.as_deref(),
        generation,
        token.as_deref(),
        length,
    )
    .map_err(Failure::Rejected)
}
