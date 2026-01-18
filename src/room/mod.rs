#![allow(unused_imports)]

mod create;
mod discovery;
mod model;
mod naming;
mod post_create;
mod remove;
mod rename;

pub use create::{create_room, CreateRoomError, CreateRoomOptions};
pub use discovery::{discover_rooms, DiscoveryError};
pub use model::{RoomInfo, RoomStatus};
pub use naming::generate_room_name;
pub use post_create::{run_post_create_commands, PostCreateHandle, PostCreateResult};
pub use remove::{remove_room, DirtyStatus, RemoveRoomError};
pub use rename::{rename_room, RenameRoomError};
