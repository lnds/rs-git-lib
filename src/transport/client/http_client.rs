use url::Url;
use std::io::{Error, ErrorKind, Result as IOResult};


use crate::transport::client::Protocol;
use crate::packfile::refs::{Refs, Ref};
use super::packet::{read_packet_line, read_flush_packet, receive, parse_refs_lines, GIT_UPLOAD_PACK_HEADER, GIT_FLUSH_HEADER};

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

const REF_DISCOVERY_ENDPOINT: &str = "/info/refs?service=git-upload-pack";

impl Protocol for HttpProtocol {
    fn discover_refs(&mut self) -> IOResult<Refs> {
        let discovery_url = format!("{}{}", self.url.as_str(), REF_DISCOVERY_ENDPOINT);
        let mut res = reqwest::blocking::get(&discovery_url)
            .map_err(|e| std::io::Error::new(ErrorKind::Other, e))?;
        let status = res.status();
        if !status.is_success() {
            return Err(Error::new(
                ErrorKind::Other,
                &format!("HTTP ERROR: {}", status.as_u16())[..],
            ));
        }

        let first = read_packet_line(&mut res)?.unwrap_or_else(|| vec![]);
        if first != GIT_UPLOAD_PACK_HEADER {
            return Err(Error::new(ErrorKind::Other, "flush not received"));
        }

        let flush = read_flush_packet(&mut res)?.unwrap();
        if flush != GIT_FLUSH_HEADER {
            return Err(Error::new(ErrorKind::Other, "flush not received"));
        }
        parse_refs_lines(&receive(&mut res)?)
    }

    fn fetch_packfile(&mut self, _reference: &[Ref]) -> IOResult<Vec<u8>> {
        unimplemented!()
    }

    fn protocol(&self) -> &'static str {
        "ssh-protocol"
    }
}
