use url::Url;
use std::io::Result as IOResult;

use crate::transport::client::Protocol;
use crate::packfile::refs::{Refs, Ref};

#[derive(Debug)]
pub struct GitProtocol {
    url: Url,
}

impl GitProtocol {
    pub fn new(url: &Url) -> Self {
        GitProtocol { url: url.clone() }
    }
}

impl Protocol for GitProtocol {
    fn discover_refs(&mut self) -> IOResult<Refs> {
        unimplemented!()
    }

    fn fetch_packfile(&mut self, _reference: &[Ref]) -> IOResult<Vec<u8>> {
        unimplemented!()
    }

    fn protocol(&self) -> &'static str {
        "git-protocol"
    }
}
