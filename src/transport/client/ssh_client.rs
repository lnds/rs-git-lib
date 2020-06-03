use url::Url;
use std::io::Result as IOResult;


use crate::transport::client::Protocol;
use crate::packfile::refs::{Refs, Ref};

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

    fn fetch_packfile(&mut self, _reference: &[Ref]) -> IOResult<Vec<u8>> {
        unimplemented!()
    }

    fn protocol(&self) -> &'static str {
        "ssh-protocol"
    }
}
