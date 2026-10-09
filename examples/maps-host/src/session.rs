use crate::{ApprovedPoint, read_points};
use rom::{Actor, Key, Runtime};
use rom_map_core::{Error, RequestContext, Result};
/// Host-owned session checks must read current state, not capture an initial grant.
/// The actor and selected kind remain host-owned. Revocation discards the entire snapshot.
pub async fn read_session_points(
    runtime: &Runtime,
    actor: &Actor,
    kind: &str,
    keys: &[Key],
    context: &RequestContext,
    mut authorize: impl FnMut() -> bool,
) -> Result<Vec<ApprovedPoint>> {
    context
        .run(async {
            if !authorize() {
                return Err(Error::Rejected);
            }
            let outcome = read_points(runtime, actor, kind, keys, context).await;
            if !authorize() {
                return Err(Error::Rejected);
            }
            outcome
        })
        .await
}
