//! Session logic shared by the CLI, audio worker, and desktop shell.

mod boundary;
mod protocol;
mod session;

pub use protocol::*;
pub use session::{Policy, Session, Work};
