use super::traits::PersistentId;
use std::fmt;
use uuid::Uuid;

/// A bare UUID-backed persistent id.
///
/// Usable directly, though most call sites will prefer a distinct newtype
/// (`AssetId`, `SceneId`, ...) defined via [`define_persistent_id!`] so the
/// type system stops different kinds of persistent id from being mixed up.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct UuidId(Uuid);

impl UuidId {
    /// Function that creates UuidId
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
    /// Function that returns self from uuid
    pub const fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }
    /// Function that returns nil
    pub const fn nil() -> Self {
        Self(Uuid::nil())
    }
    /// Function that returns Uuid
    pub const fn uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for UuidId {
    fn default() -> Self {
        Self::new()
    }
}

impl PersistentId for UuidId {
    fn uuid(&self) -> Uuid {
        self.0
    }
}

impl fmt::Display for UuidId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Defines a distinct, UUID-backed persistent id newtype.
///
/// Two ids produced by two different invocations of this macro are
/// different Rust types, even though both just wrap a `Uuid` underneath —
/// so an `AssetId` can never be passed where a `SceneId` is expected.
///
/// ```ignore
/// ccore::define_persistent_id!(AssetId);
/// ccore::define_persistent_id!(
///     /// Persistent identifier for a scene.
///     SceneId
/// );
/// ```
#[macro_export]
macro_rules! define_persistent_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
        pub struct $name($crate::__private::Uuid);

        impl $name {
            /// Function that creates uuid for impl
            pub fn new() -> Self {
                Self($crate::__private::Uuid::new_v4())
            }

            pub const fn from_uuid(uuid: $crate::__private::Uuid) -> Self {
                Self(uuid)
            }

            pub const fn nil() -> Self {
                Self($crate::__private::Uuid::nil())
            }

            pub const fn uuid(&self) -> $crate::__private::Uuid {
                self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl $crate::id::PersistentId for $name {
            fn uuid(&self) -> $crate::__private::Uuid {
                self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

#[cfg(test)]
#[allow(dead_code)]
mod tests {
    crate::define_persistent_id!(TestAssetId);
    crate::define_persistent_id!(TestSceneId);

    #[test]
    fn distinct_ids_are_distinct_types_but_same_representation() {
        let asset = TestAssetId::new();
        let scene = TestSceneId::new();
        // Different types entirely - this is a compile-time guarantee,
        // demonstrated here just by the fact both independently compile
        // and round-trip through their own uuid.
        assert_eq!(TestAssetId::from_uuid(asset.uuid()), asset);
        assert_eq!(TestSceneId::from_uuid(scene.uuid()), scene);
    }

    #[test]
    fn nil_is_stable() {
        assert_eq!(TestAssetId::nil(), TestAssetId::nil());
    }
}
