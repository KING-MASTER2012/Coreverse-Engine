//! `use ccore::prelude::*;` to pull in the commonly used types from every
//! module in one line.

pub use crate::build_info::BuildInfo;
pub use crate::error::{CoreError, CoreResult};
pub use crate::handle::{Handle, WeakHandle};
pub use crate::id::{
    AssetId, GenerationId, Id, PersistentId, PluginId, ProjectId, RuntimeId, SceneId, UuidId,
};
pub use crate::platform::{Architecture, Endian, Platform};
pub use crate::traits::{Disposable, Identifiable, Initialize, Named, Reset, Shutdown, Validate};
pub use crate::types::{ByteSize, Color, FrameNumber, Rect, Timestamp};
pub use crate::version::Version;
