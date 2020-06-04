pub(crate) mod local_client;
pub(crate) mod file_client;
pub(crate) mod ssh_client;
pub(crate) mod git_client;
pub(crate) mod http_client;
pub(crate) mod packet;

use crate::packfile::refs::{Ref, Refs};
use std::io::Result as IOResult;

pub trait Protocol {
    fn discover_refs(&mut self) -> IOResult<Refs>;
    fn fetch_packfile(&mut self, wants: &[Ref]) -> IOResult<Vec<u8>>;
    fn protocol(&self) -> &'static str;
}
