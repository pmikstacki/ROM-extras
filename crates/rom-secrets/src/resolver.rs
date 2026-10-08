use crate::{Error, ResolvedSecret, SecretRef, Version};
/// Host resolver. Implementations enforce approved mappings, bounds and deadlines.
/// No method mutates a Resource or returns a raw provider response.
pub trait SecretResolver: Send + Sync {
    /// Resolve an approved reference with explicit version selection.
    fn resolve<'a>(
        &'a self,
        reference: &'a SecretRef,
        version: Version,
    ) -> impl std::future::Future<Output = Result<ResolvedSecret, Error>> + Send + 'a;
}
