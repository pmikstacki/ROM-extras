//! Isolated mutable test authority; changing it never mutates Resource revisions.
use rom::Actor;
use std::sync::{Arc, atomic::AtomicBool};
// Each fixture actor owns isolated mutable host policy; native Resource revisions stay unchanged.
type Permission = (Arc<AtomicBool>, Arc<AtomicBool>);
fn registry() -> &'static std::sync::Mutex<std::collections::BTreeMap<String, Permission>> {
    static REGISTRY: std::sync::OnceLock<
        std::sync::Mutex<std::collections::BTreeMap<String, Permission>>,
    > = std::sync::OnceLock::new();
    REGISTRY.get_or_init(Default::default)
}
pub(crate) fn permission(actor: &Actor) -> Option<Permission> {
    registry().lock().unwrap().get(&actor.subject).cloned()
}
pub(crate) struct Permissions(Actor);
impl Permissions {
    pub(crate) fn new(actor: Actor, row: Arc<AtomicBool>, field: Arc<AtomicBool>) -> Self {
        assert!(
            registry()
                .lock()
                .unwrap()
                .insert(actor.subject.clone(), (row, field))
                .is_none()
        );
        Self(actor)
    }
}
impl Drop for Permissions {
    fn drop(&mut self) {
        registry().lock().unwrap().remove(&self.0.subject);
    }
}
