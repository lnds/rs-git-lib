use reqwest::header::CONTENT_TYPE;
use std::io::{BufReader, Error, ErrorKind, Result as IOResult};
use url::Url;

use super::packet::{
    create_packfile_negotiation_request, parse_refs_lines, read_flush_packet, read_packet_line,
    receive_packet, receive_packet_file_with_sideband, GIT_FLUSH_HEADER, GIT_UPLOAD_PACK_HEADER,
};
use crate::packfile::packfile_parser::PackFileParser;
use crate::packfile::refs::{Ref, Refs};
use crate::transport::client::Protocol;

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
const REQUIRED_CAPABILTIES: [&str; 3] = ["multi_ack_detailed", "side-band-64k", "agent=git/1.8.1"];
const UPLOAD_PACK_ENDPOINT: &str = "/git-upload-pack";

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
        parse_refs_lines(&receive_packet(&mut res)?)
    }

    fn fetch_packfile(&mut self, refs: &[Ref]) -> IOResult<PackFileParser> {
        self.client = Client::new();
        let body = create_packfile_negotiation_request(&REQUIRED_CAPABILTIES, refs);
        let pack_endpoint = [self.url.as_str(), UPLOAD_PACK_ENDPOINT].join("");

        let res = self
            .client
            .post(&pack_endpoint)
            .header(CONTENT_TYPE, "application/x-git-upload-pack-request")
            .body(body)
            .send()
            .map_err(|e| Error::new(ErrorKind::Other, e))?;

        let mut reader = BufReader::with_capacity(16 * 1024, res);
        receive_packet_file_with_sideband(&mut reader)
    }

    fn protocol(&self) -> &'static str {
        "ssh-protocol"
    }
}
