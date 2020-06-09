pub mod client;
mod url_parser;

use crate::packfile::refs::{Ref, Refs};
use client::file_client::FileProtocol;
use client::git_client::GitProtocol;
use client::http_client::HttpProtocol;
use client::local_client::LocalProtocol;
use client::ssh_client::SshProtocol;
use client::Protocol;
use std::io::Result as IOResult;
use url_parser::UrlType::{FILE, GIT, HTTP, LOCAL, SSH};
use crate::packfile::packfile_parser::PackFileParser;

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

    pub fn fetch_packfile(&mut self, wants: &[Ref]) -> IOResult<PackFileParser> {
        self.client.fetch_packfile(wants)
    }
}
