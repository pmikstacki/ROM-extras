use rom_blob::Error;
pub(crate) fn read(error: object_store::Error) -> Error {
    match error {
        object_store::Error::NotFound { .. } => Error::Missing,
        object_store::Error::PermissionDenied { .. }
        | object_store::Error::Unauthenticated { .. } => Error::Denied,
        object_store::Error::Precondition { .. } => Error::Conflict,
        object_store::Error::NotSupported { .. } | object_store::Error::NotImplemented { .. } => {
            Error::Unsupported
        }
        _ => Error::Backend,
    }
}
pub(crate) fn write(error: object_store::Error) -> Error {
    match error {
        object_store::Error::AlreadyExists { .. } | object_store::Error::Precondition { .. } => {
            Error::Conflict
        }
        object_store::Error::PermissionDenied { .. }
        | object_store::Error::Unauthenticated { .. } => Error::Denied,
        object_store::Error::NotSupported { .. } | object_store::Error::NotImplemented { .. } => {
            Error::Unsupported
        }
        _ => Error::Unknown,
    }
}
