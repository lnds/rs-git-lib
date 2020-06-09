//! # rs-git-lib
//!
//! A Rust Native Library for Git
//!
#[macro_use]
extern crate num_derive;

mod packfile;
mod transport;
mod store;
mod utils;

use crate::packfile::refs::Refs;
use std::io::Result as IoResult;
use transport::Transport;

/// A Git Repository
pub struct Repo {
    dir: String,
    refs: Refs,
    count_objects: usize,
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
    /// let repo = Repo::clone_from("https://github.com/lnds/rs-git-lib.git", Some("/tmp/rs-git".to_string()));
    /// ```
    ///
    pub fn clone_from(url: &str, dir: Option<String>) -> IoResult<Self> {
        let mut transport = Transport::from_url(url, dir)?;
        let dir = transport.dir();
        let refs = transport.discover_refs()?;
        let mut packfile_parser = transport.fetch_packfile(&refs)?;
        let packfile = packfile_parser.parse(Some(&dir))?;
        Ok(Repo { dir, refs, count_objects: packfile_parser.count_objects() })
    }

    ///
    /// return references of cloned repo
    ///
    /// ```
    /// use rs_git_lib::Repo;
    /// let repo = Repo::clone_from("https://github.com/lnds/rs-git-lib.git", None);
    /// let refs = repo.unwrap().refs();
    /// assert_eq!(refs[0].name, "HEAD");
    /// assert_eq!(refs[1].name, "refs/heads/master");
    /// ```
    pub fn refs(self) -> Refs {
        self.refs
    }

    ///
    /// return references of cloned repo
    ///
    /// ```
    /// use rs_git_lib::Repo;
    /// let repo = Repo::clone_from("https://github.com/lnds/rs-git-lib.git", None).unwrap();
    /// assert_eq!(repo.dir(), "rs-git-lib");
    /// ```
    pub fn dir(self) -> String {
        self.dir
    }

    ///
    /// return how many objects are in the repo
    ///
    /// ```
    /// use rs_git_lib::Repo;
    /// let repo = Repo::clone_from("https://github.com/lnds/rs-git-lib.git", None).unwrap();
    /// assert!(repo.count_objects() > 1);
    /// let repo = Repo::clone_from("https://github.com/lnds/redondeo.git", Some("/tmp/redondeo".to_string())).unwrap();
    /// assert_eq!(repo.count_objects(), 25);
    /// ```
    pub fn count_objects(self) -> usize {
        self.count_objects
    }
}
