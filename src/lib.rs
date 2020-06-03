//! # rs-git-lib
//!
//! A Rust Native Library for Git
//!
mod packfile;
mod transport;


use std::io::Result as IoResult;
use transport::Transport;
use crate::packfile::refs::Refs;
use std::path::PathBuf;

/// A Git Repository
pub struct Repo {
    dir: String,
    refs: Refs,
}

impl Repo {

    /// clone a git repo
    /// # Arguments
    ///
    /// * `url` - a string that holds de repo url from where we will clone
    /// * `dir` - an optional string with the path where the cloned repo will be out.
    /// If None the dir wil be created based on url.
    ///
    /// # Examples
    ///
    /// ```
    /// // will write the repo on rs-git-lib directory
    /// use rs_git_lib::Repo;
    /// let repo = Repo::clone_from("https://github.com/lnds/rs-git-lib.git", None);
    /// ```
    ///
    /// ```
    /// // will write the repo on /tmp/rs-git directory
    /// use rs_git_lib::Repo;
    ///  let repo = Repo::clone_from("https://github.com/lnds/rs-git-lib.git", Some("/tmp/rs-git".to_string()));
    /// ```
    ///
    pub fn clone_from(url: &str, dir: Option<String>) -> IoResult<Self> {
        let mut transport = Transport::from_url(url, dir)?;
        let dir = transport.dir();
        let refs = transport.discover_refs()?;
        Ok(Repo {
            dir,
            refs
        })
    }


}

