use url::Url;
use std::io::Result as IOResult;


use crate::transport::client::Protocol;
use crate::packfile::refs::{Refs, Ref};

type Client = reqwest::blocking::Client;

#[derive(Debug)]
pub struct HttpProtocol {
    url: Url,
    client: Client,

}

impl HttpProtocol {
    pub fn new(url: &Url) -> Self {
        HttpProtocol {
            url: url.clone(),
            client: Client::new(),
        }
    }
}

impl Protocol for HttpProtocol {
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
