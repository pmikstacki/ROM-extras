use crate::{Connection, ControlTable, Value};
use rom_sql_core::{OwnerError, OwnerState, OwnerTransaction};
/// Native owner transaction; caught failures poison all later writes and commit.
/// Drop attempts native full rollback; unknown cleanup permanently retires the connection.
pub struct Transaction<'a> {
    connection: &'a mut Connection,
    table: &'a ControlTable,
    lock: u64,
    locked: bool,
    active: bool,
    failed: bool,
}
impl<'a> Transaction<'a> {
    pub(crate) fn new(connection: &'a mut Connection, table: &'a ControlTable, lock: u64) -> Self {
        Self {
            connection,
            table,
            lock,
            locked: false,
            active: true,
            failed: false,
        }
    }
    /// Execute fixed trusted DML under the same lock, with bounded owned native binds.
    /// SQL through16KiB,64 binds,32767bytes per variable bind and64KiB total. No DDL/PLSQL/rowsets/RETURNING.
    /// Host-owned schema/routines must not introduce external effects or transaction control.
    pub fn execute(&mut self, sql: &str, params: &[Value]) -> Result<u64, OwnerError> {
        let size = params.iter().try_fold(0_usize, |sum, v| {
            sum.checked_add(v.size()?).ok_or(OwnerError::Invalid)
        });
        if self.failed
            || !self.locked
            || !self.active
            || sql.is_empty()
            || sql.len() > 16 * 1024
            || params.len() > 64
            || !size.is_ok_and(|n| n <= 65536)
        {
            self.failed = true;
            return Err(OwnerError::Invalid);
        }
        let sql = sql.to_owned();
        let params = params.to_vec();
        let result = self.connection.call(move |w| w.execute(sql, params));
        if result.is_err() {
            self.failed = true;
        }
        result
    }
}
impl OwnerTransaction for Transaction<'_> {
    fn lock_owner(&mut self) -> Result<OwnerState, OwnerError> {
        if self.failed || self.locked {
            self.failed = true;
            return Err(OwnerError::Invalid);
        }
        let table = self.table.clone();
        let lock = self.lock;
        let result = self.connection.call(move |w| w.read_owner(table, lock));
        if result.is_ok() {
            self.locked = true;
        } else {
            self.failed = true;
        }
        result
    }
    fn write_owner(&mut self, state: OwnerState) -> Result<(), OwnerError> {
        let generation = i64::try_from(state.generation()).map_err(|_| {
            self.failed = true;
            OwnerError::Invalid
        })?;
        let token = state.token().map(|t| t.as_bytes().to_vec());
        let sql = format!(
            "UPDATE {} SET generation=:1,owner_token=:2 WHERE singleton_key=:3 AND format_version=1 AND store_identity=:4",
            self.table.qualified
        );
        let affected = self.execute(
            &sql,
            &[
                Value::Integer(Some(generation)),
                Value::Raw(token),
                Value::Integer(Some(1)),
                Value::Raw(Some(self.table.identity.to_vec())),
            ],
        )?;
        if affected != 1 {
            self.failed = true;
            return Err(OwnerError::Invalid);
        }
        Ok(())
    }
    fn commit(mut self) -> Result<(), OwnerError> {
        if self.failed || !self.locked {
            let result = self.connection.cleanup();
            self.active = false;
            result?;
            return Err(OwnerError::Invalid);
        }
        let result = self.connection.call(|w| w.terminal(true));
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
