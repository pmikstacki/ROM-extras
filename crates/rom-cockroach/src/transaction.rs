use crate::{Connection, ControlTable};
use rom_sql_core::{OwnerError, OwnerState, OwnerTransaction, validate_owner_row};
use tokio_postgres::types::ToSql;
/// Native transaction retaining its control-row lock through complete terminal response.
/// Drop attempts bounded rollback; uncertain cleanup retires the native connection.
pub struct Transaction<'a> {
    connection: &'a mut Connection,
    table: &'a ControlTable,
    locked: bool,
    active: bool,
    failed: bool,
}
impl<'a> Transaction<'a> {
    pub(crate) fn new(connection: &'a mut Connection, table: &'a ControlTable) -> Self {
        Self {
            connection,
            table,
            locked: false,
            active: true,
            failed: false,
        }
    }
    /// Execute trusted prepared persistence after shared ownership validation.
    /// Fixed SQL is capped at16KiB and64parameters. The host must bound parameter bytes and total work.
    /// Never change session/transaction state, run DDL, emit rowsets or invoke external effects.
    /// This is not an application action API. A caught error permanently poisons this transaction.
    pub fn execute(
        &mut self,
        sql: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<u64, OwnerError> {
        if self.failed
            || !self.locked
            || !self.active
            || sql.is_empty()
            || sql.len() > 16 * 1024
            || params.len() > 64
        {
            self.failed = true;
            return Err(OwnerError::Invalid);
        }
        let result = self.connection.execute(sql, params);
        if result.is_err() {
            self.failed = true;
        }
        result
    }
    fn read_owner(&mut self) -> Result<OwnerState, OwnerError> {
        let sql = format!(
            "SELECT format_version, CASE WHEN octet_length(store_identity)=32 THEN store_identity ELSE NULL END, generation, CASE WHEN octet_length(owner_token)=32 THEN owner_token ELSE NULL END, octet_length(owner_token) FROM {} WHERE singleton_key=$1 LIMIT 2 FOR UPDATE",
            self.table.qualified
        );
        let rows = self.connection.query(&sql, &[&1_i32])?;
        if rows.is_empty() {
            return Err(OwnerError::Uninitialized);
        }
        if rows.len() != 1 {
            return Err(OwnerError::Invalid);
        }
        let row = &rows[0];
        if row.try_get::<_, i32>(0).map_err(|_| OwnerError::Invalid)? != 1 {
            return Err(OwnerError::UnsupportedFormat);
        }
        validate_owner_row(
            1,
            &self.table.identity,
            row.try_get::<_, Option<&[u8]>>(1)
                .map_err(|_| OwnerError::Invalid)?,
            row.try_get::<_, i64>(2).map_err(|_| OwnerError::Invalid)?,
            row.try_get::<_, Option<&[u8]>>(3)
                .map_err(|_| OwnerError::Invalid)?,
            row.try_get::<_, Option<i64>>(4)
                .map_err(|_| OwnerError::Invalid)?,
        )
    }
}
impl OwnerTransaction for Transaction<'_> {
    fn lock_owner(&mut self) -> Result<OwnerState, OwnerError> {
        if self.failed || self.locked {
            self.failed = true;
            return Err(OwnerError::Invalid);
        }
        let result = self.read_owner();
        if result.is_ok() {
            self.locked = true;
        } else {
            self.failed = true;
        }
        result
    }
    fn write_owner(&mut self, state: OwnerState) -> Result<(), OwnerError> {
        let generation = i64::try_from(state.generation()).map_err(|_| OwnerError::Invalid)?;
        let token = state.token();
        let bytes = token.as_ref().map(|t| &t.as_bytes()[..]);
        let sql = format!(
            "UPDATE {} SET generation=$1,owner_token=$2 WHERE singleton_key=$3 AND format_version=1 AND store_identity=$4",
            self.table.qualified
        );
        let affected = self.execute(
            &sql,
            &[&generation, &bytes, &1_i32, &&self.table.identity[..]],
        )?;
        if affected != 1 {
            self.failed = true;
            return Err(OwnerError::Invalid);
        }
        Ok(())
    }
    fn commit(mut self) -> Result<(), OwnerError> {
        if self.failed || !self.locked {
            self.connection.cleanup()?;
            self.active = false;
            return Err(OwnerError::Invalid);
        }
        let result = self.connection.batch("COMMIT");
        self.active = false;
        if result.is_err() {
            self.connection.retire();
            return Err(OwnerError::Unknown);
        }
        Ok(())
    }
    fn rollback(mut self) -> Result<(), OwnerError> {
        let result = self.connection.cleanup();
        self.active = false;
        result
    }
}
impl Drop for Transaction<'_> {
    fn drop(&mut self) {
        if self.active {
            let _ = self.connection.cleanup();
        }
    }
}
impl std::fmt::Debug for Transaction<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Transaction { redacted }")
    }
}
