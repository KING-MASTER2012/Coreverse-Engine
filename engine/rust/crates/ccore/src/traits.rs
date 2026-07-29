/// A type that can report a human-readable name.
pub trait Named {
    /// Function that returns name
    fn name(&self) -> &str;
}

/// A type that is associated with an [`Id`](crate::id::Id) of some kind.
pub trait Identifiable {
    /// ID parameter
    type Id: crate::id::Id;
    /// Function that returns ID
    fn id(&self) -> Self::Id;
}

/// A type that can validate its own internal invariants, e.g. after being
/// deserialized or mutated through an unchecked path.
pub trait Validate {
    /// Error parameter
    type Error;
    /// Function that validates
    fn validate(&self) -> Result<(), Self::Error>;
}

/// A type that can be reset back to a default/initial state without being
/// reallocated. Useful for pooled or reused objects.
pub trait Reset {
    /// Reset function
    fn reset(&mut self);
}

/// A type with an explicit initialization step, separate from
/// construction (e.g. because it needs resources not available in `new`).
pub trait Initialize {
    /// Error parameter
    type Error;
    /// The Initializer function
    fn initialize(&mut self) -> Result<(), Self::Error>;
}

/// A type with an explicit shutdown step, separate from `Drop` - useful
/// when shutdown can fail, needs to run in a particular order relative to
/// other systems, or needs to happen before the value is actually dropped.
pub trait Shutdown {
    /// The shutdown function
    fn shutdown(&mut self);
}

/// A type that owns a resource which must be explicitly released (GPU
/// handles, native library handles, file handles kept open long-term,
/// ...).
pub trait Disposable {
    /// The Dispose function
    fn dispose(&mut self);
}
