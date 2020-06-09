use std::io::Result as IOResult;

use crate::packfile::refs::{Ref, Refs};
use crate::transport::client::Protocol;
use crate::packfile::packfile_parser::PackFileParser;

#[derive(Debug)]
pub struct LocalProtocol {
    path: String,
}

impl LocalProtocol {
    pub fn new(path: String) -> Self {
        LocalProtocol { path }
    }
}

impl Protocol for LocalProtocol {
    fn discover_refs(&mut self) -> IOResult<Refs> {
        unimplemented!()
    }

    fn fetch_packfile(&mut self, _reference: &[Ref]) -> IOResult<PackFileParser> {
        unimplemented!()
    }

    fn protocol(&self) -> &'static str {
        "local-protocol"
    }
}
