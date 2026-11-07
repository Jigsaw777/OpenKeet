//! Session logic shared by the CLI, audio worker, and desktop shell.

mod boundary;
pub mod prep;
mod protocol;
mod session;

pub use protocol::*;
pub use session::{Policy, Session, Work};
