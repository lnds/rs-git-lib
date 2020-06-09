pub(crate) mod file_client;
pub(crate) mod git_client;
pub(crate) mod http_client;
pub(crate) mod local_client;
pub(crate) mod packet;
pub(crate) mod ssh_client;

use crate::packfile::refs::{Ref, Refs};
use std::io::Result as IOResult;
use crate::packfile::packfile_parser::PackFileParser;

pub trait Protocol {
    fn discover_refs(&mut self) -> IOResult<Refs>;
    fn fetch_packfile(&mut self, wants: &[Ref]) -> IOResult<PackFileParser>;
    fn protocol(&self) -> &'static str;
}
