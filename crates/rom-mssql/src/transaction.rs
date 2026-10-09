use crate::{Connection, ControlTable};
use rom_sql_core::{OwnerError, OwnerState, OwnerToken, OwnerTransaction};
/// One native transaction. All protected persistence retains its control-row lock.
/// Drop attempts bounded complete rollback; uncertain cleanup retires the connection.
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
    /// Execute prepared native persistence after core ownership validation and locking.
    ///
    /// This trusted driver extension is not an application action hook. Use fixed SQL and bound
    /// parameters. Never change transaction/session state, emit rowsets, run DDL or call external
    /// effects. The extras-owned control table must have no triggers. SQL is capped at 16 KiB,
    /// parameters at 64. The host must bound parameter sizes and native statement work.
    /// An error poisons the transaction for commit even if the caller catches it.
    pub fn execute(
        &mut self,
        sql: &str,
        params: &[&dyn tiberius::ToSql],
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
        let result = self.connection.run(async |c| c.execute(sql, params).await);
        let result = result.and_then(|result| {
            result.rows_affected().iter().try_fold(0_u64, |sum, n| {
                sum.checked_add(*n).ok_or(OwnerError::Invalid)
            })
        });
        if result.is_err() {
            self.failed = true;
        }
        result
    }
}
impl OwnerTransaction for Transaction<'_> {
    fn lock_owner(&mut self) -> Result<OwnerState, OwnerError> {
        if self.failed || self.locked {
            return Err(OwnerError::Invalid);
        }
        // CASE guards prevent oversized binary values from entering result buffers.
        let sql = format!(
            "SELECT TOP(2) format_version, CASE WHEN DATALENGTH(store_identity)=32 THEN CAST(store_identity AS VARBINARY(32)) ELSE NULL END, generation, CASE WHEN DATALENGTH(owner_token)=32 THEN CAST(owner_token AS VARBINARY(32)) ELSE NULL END, CAST(DATALENGTH(owner_token) AS BIGINT) FROM {} WITH (UPDLOCK,HOLDLOCK) WHERE singleton_key=@P1",
            self.table.qualified
        );
        let rows = self.connection.query(&sql, &[&1_i32])?;
        if rows.len() != 1 || rows[0].len() > 1 {
            return Err(OwnerError::Invalid);
        }
        let row = rows[0].first().ok_or(OwnerError::Uninitialized)?;
        if row.len() != 5 {
            return Err(OwnerError::Invalid);
        }
        if row.try_get::<i32, _>(0).map_err(|_| OwnerError::Invalid)? != Some(1) {
            return Err(OwnerError::UnsupportedFormat);
        }
        if row
            .try_get::<&[u8], _>(1)
            .map_err(|_| OwnerError::Invalid)?
            != Some(&self.table.identity[..])
        {
            return Err(OwnerError::Invalid);
        }
        let generation = u64::try_from(
            row.try_get::<i64, _>(2)
                .map_err(|_| OwnerError::Invalid)?
                .ok_or(OwnerError::Invalid)?,
        )
        .map_err(|_| OwnerError::Invalid)?;
        // Cast the length to BIGINT for both bounded and MAX host column types.
        let length = row.try_get::<i64, _>(4).map_err(|_| OwnerError::Invalid)?;
        let token = match length {
            None => None,
            Some(32) => Some(OwnerToken::new(
                row.try_get::<&[u8], _>(3)
                    .map_err(|_| OwnerError::Invalid)?
                    .ok_or(OwnerError::Invalid)?
                    .try_into()
                    .map_err(|_| OwnerError::Invalid)?,
            )?),
            _ => return Err(OwnerError::Invalid),
        };
        let state = OwnerState::new(generation, token)?;
        self.locked = true;
        Ok(state)
    }
    fn write_owner(&mut self, state: OwnerState) -> Result<(), OwnerError> {
        if self.failed || !self.locked {
            return Err(OwnerError::Invalid);
        }
        let generation = i64::try_from(state.generation()).map_err(|_| OwnerError::Invalid)?;
        let token = state.token();
        let bytes = token.as_ref().map(|t| &t.as_bytes()[..]);
        let sql = format!(
            "UPDATE {} SET generation=@P1, owner_token=@P2 WHERE singleton_key=@P3 AND format_version=1 AND store_identity=@P4",
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
        let result = self.connection.run(async |c| c.commit_transaction().await);
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
