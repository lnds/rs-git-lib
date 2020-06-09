use std::io::Result as IOResult;
use url::Url;

use crate::packfile::refs::{Ref, Refs};
use crate::transport::client::Protocol;
use crate::packfile::packfile_parser::PackFileParser;

#[derive(Debug)]
pub struct SshProtocol {
    url: Url,
}

impl SshProtocol {
    pub fn new(url: &Url) -> Self {
        SshProtocol { url: url.clone() }
    }
}

impl Protocol for SshProtocol {
    fn discover_refs(&mut self) -> IOResult<Refs> {
        unimplemented!()
    }

    fn fetch_packfile(&mut self, _reference: &[Ref]) -> IOResult<PackFileParser> {
        unimplemented!()
    }

    fn protocol(&self) -> &'static str {
        "ssh-protocol"
    }
}
