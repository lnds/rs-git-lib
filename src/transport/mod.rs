
mod url_parser;
pub mod client;

use std::io::Result as IOResult;
use url_parser::UrlType::{LOCAL, FILE, GIT, HTTP, SSH};
use client::Protocol;
use client::local_client::LocalProtocol;
use client::file_client::FileProtocol;
use client::ssh_client::SshProtocol;
use client::git_client::GitProtocol;
use client::http_client::HttpProtocol;
use crate::packfile::refs::{Refs, Ref};

pub struct Transport {
    client: Box<dyn Protocol>,
    output_dir: String,
}

impl Transport {
    pub fn from_url(repo_url: &str, dir: Option<String>) -> IOResult<Self> {
        let res = url_parser::parse(repo_url, dir)?;

        let (client, output_dir) = match res {
            LOCAL(path, dir) => (Box::new(LocalProtocol::new(path)) as Box<dyn Protocol>, dir),
            FILE(url, dir) => (Box::new(FileProtocol::new(url)) as Box<dyn Protocol>, dir),
            GIT(url, dir) => (Box::new(GitProtocol::new(&url)) as Box<dyn Protocol>, dir),
            HTTP(url, dir) => (Box::new(HttpProtocol::new(&url)) as Box<dyn Protocol>, dir),
            SSH(url, dir) => (Box::new(SshProtocol::new(&url)) as Box<dyn Protocol>, dir),
        };

        Ok(Transport { client, output_dir })
    }

    pub fn dir(&self) -> String {
        self.output_dir.to_string()
    }

    pub fn discover_refs(&mut self) -> IOResult<Refs> {
        self.client.discover_refs()
    }

    pub fn fetch_packfile(&mut self, wants: &[Ref]) -> IOResult<Vec<u8>> {
        self.client.fetch_packfile(wants)
    }
}