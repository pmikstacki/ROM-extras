use crate::{Connection, ControlTable, Value};
use rom_sql_core::{OwnerError, OwnerState, OwnerToken, OwnerTransaction};
/// Native transaction retaining its singleton lock until acknowledged completion.
/// A caught error poisons the transaction. Drop attempts bounded rollback.
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
    /// Execute trusted prepared persistence under shared ownership validation.
    /// Limits:16KiB SQL,64 parameters,60KiB parameter budget including scalar reserves.
    /// Never supply DDL, session/transaction control, rowsets or external effects.
    pub fn execute(&mut self, sql: &str, params: Vec<Value>) -> Result<u64, OwnerError> {
        let bytes = params.iter().try_fold(0usize, |n, v| {
            n.checked_add(match v {
                Value::Bytes(b) => b.len().saturating_add(9),
                _ => 16,
            })
        });
        if self.failed
            || !self.active
            || !self.locked
            || sql.is_empty()
            || sql.len() > 16384
            || params.len() > 64
            || !bytes.is_some_and(|n| n <= 61440)
        {
            self.failed = true;
            return Err(OwnerError::Invalid);
        }
        let sql = sql.to_owned();
        let result = self
            .connection
            .call(move |worker| worker.execute(sql, params));
        if result.is_err() {
            self.failed = true;
        }
        result
    }
    fn read_owner(&mut self) -> Result<OwnerState, OwnerError> {
        let sql = format!(
            "SELECT format_version, CASE WHEN OCTET_LENGTH(store_identity)=32 THEN store_identity ELSE NULL END, generation, CASE WHEN OCTET_LENGTH(owner_token)=32 THEN owner_token ELSE NULL END, OCTET_LENGTH(owner_token) FROM {} WHERE singleton_key=? LIMIT 2 FOR UPDATE",
            self.table.qualified
        );
        let rows = self
            .connection
            .call(move |worker| worker.query(sql, vec![1_i32.into()], 5, 2))?;
        if rows.is_empty() {
            return Err(OwnerError::Uninitialized);
        }
        if rows.len() != 1 {
            return Err(OwnerError::Invalid);
        }
        let row = &rows[0];
        // Accept native integer types only. Do not coerce strings, floats or oversized unsigned values.
        let integer = |i| match row.as_ref(i) {
            Some(Value::Int(v)) => Ok(*v),
            Some(Value::UInt(v)) => i64::try_from(*v).map_err(|_| OwnerError::Invalid),
            _ => Err(OwnerError::Invalid),
        };
        if integer(0)? != 1 {
            return Err(OwnerError::UnsupportedFormat);
        }
        if !matches!(row.as_ref(1), Some(Value::Bytes(b)) if b.as_slice() == self.table.identity) {
            return Err(OwnerError::Invalid);
        }
        let generation = u64::try_from(integer(2)?).map_err(|_| OwnerError::Invalid)?;
        let token = match row.as_ref(4) {
            Some(Value::NULL) => None,
            Some(Value::Int(32) | Value::UInt(32)) => match row.as_ref(3) {
                Some(Value::Bytes(b)) => Some(OwnerToken::new(
                    b.as_slice().try_into().map_err(|_| OwnerError::Invalid)?,
                )?),
                _ => return Err(OwnerError::Invalid),
            },
            _ => return Err(OwnerError::Invalid),
        };
        let state = OwnerState::new(generation, token)?;
        // Target SELECT retains metadata lock through the transaction, before engine inspection.
        let params = vec![
            self.table.names.schema().into(),
            self.table.names.table().into(),
        ];
        let rows = self.connection.call(move |worker| worker.query("SELECT ENGINE FROM information_schema.TABLES WHERE TABLE_SCHEMA=? AND TABLE_NAME=? LIMIT 2".into(), params, 1, 2))?;
        if rows.len() != 1 || !matches!(rows[0].as_ref(0), Some(Value::Bytes(b)) if b == b"InnoDB")
        {
            return Err(OwnerError::Invalid);
        }
        Ok(state)
    }
}
impl OwnerTransaction for Transaction<'_> {
    fn lock_owner(&mut self) -> Result<OwnerState, OwnerError> {
        if self.failed || self.locked || !self.active {
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
        let generation = match i64::try_from(state.generation()) {
            Ok(n) => n,
            Err(_) => {
                self.failed = true;
                return Err(OwnerError::Invalid);
            }
        };
        let token = state
            .token()
            .map_or(Value::NULL, |t| Value::Bytes(t.as_bytes().to_vec()));
        let sql = format!(
            "UPDATE {} SET generation=?,owner_token=? WHERE singleton_key=? AND format_version=1 AND store_identity=?",
            self.table.qualified
        );
        let count = self.execute(
            &sql,
            vec![
                generation.into(),
                token,
                1_i32.into(),
                self.table.identity.to_vec().into(),
            ],
        )?;
        if count != 1 {
            self.failed = true;
            return Err(OwnerError::Invalid);
        }
        Ok(())
    }
    fn commit(mut self) -> Result<(), OwnerError> {
        self.active = false;
        if self.failed || !self.locked {
            self.connection.cleanup()?;
            return Err(OwnerError::Invalid);
        }
        if self
            .connection
            .call(|worker| worker.terminal(true))
            .is_err()
        {
            self.connection.retire();
            return Err(OwnerError::Unknown);
        }
        Ok(())
    }
    fn rollback(mut self) -> Result<(), OwnerError> {
        self.active = false;
        self.connection.cleanup()
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
