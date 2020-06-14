//! # rs-git-lib
//!
//! A Rust Native Library for Git
//!
#[macro_use]
extern crate num_derive;
#[macro_use]
extern crate nom;

mod delta;
mod packfile;
mod store;
mod transport;
mod utils;

use crate::packfile::refs::{create_refs, resolve_ref, update_head, Refs};
use crate::packfile::PackFile;
use crate::store::commit::Commit;
use crate::store::object::{GitObject, GitObjectType};
use crate::store::tree::{EntryMode, Tree, TreeEntry};
use crate::utils::sha1_hash;
use byteorder::{BigEndian, WriteBytesExt};
use rustc_serialize::hex::FromHex;
use std::fs;
use std::fs::{File, Permissions};
use std::io::{Error, ErrorKind, Result as IOResult, Write};
use std::iter::FromIterator;
use std::os::unix::fs::MetadataExt;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use transport::Transport;

/// A Git Repository
pub struct Repo {
    dir: String,
    refs: Refs,
    count_objects: usize,
    pack: Option<PackFile>,
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
    /// // will write the repo on /tmp/rs-git directory
    /// use rs_git_lib::Repo;
    /// let repo = Repo::clone_from("https://github.com/lnds/rs-git-lib.git", Some("/tmp/rs-git".to_string()));
    /// ```
    ///
    pub fn clone_from(url: &str, dir: Option<String>) -> IOResult<Self> {
        let mut transport = Transport::from_url(url, dir)?;
        let dir = transport.dir();
        let refs = transport.discover_refs()?;
        let mut packfile_parser = transport.fetch_packfile(&refs)?;
        let packfile = packfile_parser.parse(Some(&dir), None)?;
        packfile.write(&dir)?;
        create_refs(&dir, &refs)?;
        update_head(&dir, &refs)?;
        let repo = Repo {
            dir,
            refs,
            count_objects: packfile_parser.count_objects(),
            pack: Some(packfile),
        };
        repo.checkout_head()?;
        Ok(repo)
    }

    ///
    /// return references of cloned repo
    ///
    /// ```
    /// use rs_git_lib::Repo;
    /// let repo = Repo::clone_from("https://github.com/lnds/rs-git-lib.git", Some("/tmp/rs-git".to_string()));
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
    /// let repo = Repo::clone_from("https://github.com/lnds/rs-git-lib.git", Some("/tmp/rs-git".to_string())).unwrap();
    /// assert_eq!(repo.dir(), "/tmp/rs-git");
    /// ```
    pub fn dir(self) -> String {
        self.dir
    }

    ///
    /// return how many objects are in the repo
    ///
    /// ```
    /// use rs_git_lib::Repo;
    /// let repo = Repo::clone_from("https://github.com/lnds/rs-git-lib.git", Some("/tmp/rs-git".to_string())).unwrap();
    /// assert!(repo.count_objects() > 1);
    /// let repo = Repo::clone_from("https://github.com/lnds/redondeo.git", Some("/tmp/redondeo".to_string())).unwrap();
    /// assert_eq!(repo.count_objects(), 25);
    /// ```
    pub fn count_objects(self) -> usize {
        self.count_objects
    }

    fn checkout_head(&self) -> IOResult<()> {
        let tip = resolve_ref(&self.dir, "HEAD")?;
        let mut idx = Vec::new();
        self.walk(&tip)
            .and_then(|t| self.walk_tree(&self.dir, &t, &mut idx).ok());
        write_index(&self.dir, &mut idx[..])?;
        Ok(())
    }

    fn walk(&self, sha: &str) -> Option<Tree> {
        self.read_object(sha)
            .ok()
            .and_then(|object| match object.object_type {
                GitObjectType::Commit => object.as_commit().and_then(|c| self.extract_tree(&c)),
                GitObjectType::Tree => object.as_tree(),
                _ => None,
            })
    }

    fn walk_tree(&self, parent: &str, tree: &Tree, idx: &mut Vec<IndexEntry>) -> IOResult<()> {
        for entry in &tree.entries {
            let &TreeEntry {
                ref path,
                ref mode,
                ref sha,
            } = entry;
            let mut full_path = PathBuf::new();
            full_path.push(parent);
            full_path.push(path);
            match *mode {
                EntryMode::SubDirectory => {
                    fs::create_dir_all(&full_path)?;
                    let path_str = full_path.to_str().unwrap();
                    self.walk(sha)
                        .and_then(|t| self.walk_tree(path_str, &t, idx).ok());
                }
                EntryMode::Normal | EntryMode::Executable => {
                    let object = self.read_object(sha)?;
                    let mut file = File::create(&full_path)?;
                    file.write_all(&object.content[..])?;
                    let meta = file.metadata()?;
                    let mut perms: Permissions = meta.permissions();

                    let raw_mode = match *mode {
                        EntryMode::Normal => 33188,
                        _ => 33261,
                    };
                    perms.set_mode(raw_mode);
                    fs::set_permissions(&full_path, perms)?;

                    let idx_entry = get_index_entry(
                        &self.dir,
                        full_path.to_str().unwrap(),
                        mode.clone(),
                        sha.clone(),
                    )?;
                    idx.push(idx_entry);
                }
                ref e => panic!("Unsupported Entry Mode {:?}", e),
            }
        }
        Ok(())
    }

    pub fn read_object(&self, sha: &str) -> IOResult<GitObject> {
        // Attempt to read from disk first
        GitObject::open(&self.dir, sha).or_else(|_| {
            // If this isn't there, read from the packfile
            let pack = self
                .pack
                .as_ref()
                .ok_or_else(|| Error::new(ErrorKind::Other, "can't read pack object"))?;
            pack.find_by_sha(sha).map(|o| o.unwrap())
        })
    }

    fn extract_tree(&self, commit: &Commit) -> Option<Tree> {
        let sha = commit.tree;
        self.read_tree(sha)
    }

    fn read_tree(&self, sha: &str) -> Option<Tree> {
        self.read_object(sha).ok().and_then(|obj| obj.as_tree())
    }
}

#[derive(Debug)]
struct IndexEntry {
    ctime: i64,
    mtime: i64,
    device: i32,
    inode: u64,
    mode: u16,
    uid: u32,
    gid: u32,
    size: i64,
    sha: Vec<u8>,
    file_mode: EntryMode,
    path: String,
}

fn write_index(repo: &str, entries: &mut [IndexEntry]) -> IOResult<()> {
    let mut path = PathBuf::new();
    path.push(repo);
    path.push(".git");
    path.push("index");
    let mut idx_file = File::create(path)?;
    let encoded = encode_index(entries)?;
    idx_file.write_all(&encoded[..])?;
    Ok(())
}

fn encode_index(idx: &mut [IndexEntry]) -> IOResult<Vec<u8>> {
    let mut encoded = index_header(idx.len())?;
    idx.sort_by(|a, b| a.path.cmp(&b.path));
    let entries: Result<Vec<_>, _> = idx.iter().map(|e| encode_entry(e)).collect();
    let mut encoded_entries = entries?.concat();
    encoded.append(&mut encoded_entries);
    let mut hash = sha1_hash(&encoded);
    encoded.append(&mut hash);
    Ok(encoded)
}

fn index_header(num_entries: usize) -> IOResult<Vec<u8>> {
    let mut header = Vec::with_capacity(12);
    let magic = 1_145_655_875; // "DIRC"
    let version: u32 = 2;
    header.write_u32::<BigEndian>(magic)?;
    header.write_u32::<BigEndian>(version)?;
    header.write_u32::<BigEndian>(num_entries as u32)?;
    Ok(header)
}

fn encode_entry(entry: &IndexEntry) -> IOResult<Vec<u8>> {
    let mut buf: Vec<u8> = Vec::with_capacity(62);
    let &IndexEntry {
        ctime,
        mtime,
        device,
        inode,
        mode,
        uid,
        gid,
        size,
        ..
    } = entry;
    let &IndexEntry {
        ref sha,
        ref file_mode,
        ref path,
        ..
    } = entry;
    let flags = (path.len() & 0xFFF) as u16;
    let (encoded_type, perms) = match *file_mode {
        EntryMode::Normal | EntryMode::Executable => (8u32, mode as u32),
        EntryMode::Symlink => (10u32, 0u32),
        EntryMode::Gitlink => (14u32, 0u32),
        _ => unreachable!("Tried to create an index entry for a non-indexable object"),
    };
    let encoded_mode = (encoded_type << 12) | perms;

    let path_and_padding = {
        // This is the total length of the index entry file
        // NUL-terminated and padded with enough NUL bytes to pad
        // the entry to a multiple of 8 bytes.
        //
        // The -2 is because of the amount needed to compensate for the flags
        // only being 2 bytes.
        let mut v: Vec<u8> = Vec::from_iter(path.as_bytes().iter().cloned());
        v.push(0u8);
        let padding_size = 8 - ((v.len() - 2) % 8);
        let padding = vec![0u8; padding_size];
        if padding_size != 8 {
            v.extend(padding);
        }
        v
    };

    buf.write_u32::<BigEndian>(ctime as u32)?;
    buf.write_u32::<BigEndian>(0u32)?;
    buf.write_u32::<BigEndian>(mtime as u32)?;
    buf.write_u32::<BigEndian>(0u32)?;
    buf.write_u32::<BigEndian>(device as u32)?;
    buf.write_u32::<BigEndian>(inode as u32)?;
    buf.write_u32::<BigEndian>(encoded_mode)?;
    buf.write_u32::<BigEndian>(uid as u32)?;
    buf.write_u32::<BigEndian>(gid as u32)?;
    buf.write_u32::<BigEndian>(size as u32)?;
    buf.extend_from_slice(&sha);
    buf.write_u16::<BigEndian>(flags)?;
    buf.extend(path_and_padding);
    Ok(buf)
}

fn get_index_entry(
    root: &str,
    path: &str,
    file_mode: EntryMode,
    sha: String,
) -> IOResult<IndexEntry> {
    let file = File::open(path)?;
    let meta = file.metadata()?;

    // We need to remove the repo path from the path we save on the index entry
    // FIXME: This doesn't need to be a path since we just discard it again
    let relative_path = PathBuf::from(path.trim_start_matches(root).trim_start_matches('/'));
    let decoded_sha = sha
        .from_hex()
        .map_err(|_| Error::new(ErrorKind::Other, "can't decode sha"))?;

    Ok(IndexEntry {
        ctime: meta.ctime(),
        mtime: meta.mtime(),
        device: meta.dev() as i32,
        inode: meta.ino(),
        mode: meta.mode() as u16,
        uid: meta.uid(),
        gid: meta.gid(),
        size: meta.size() as i64,
        sha: decoded_sha,
        file_mode,
        path: relative_path.to_str().unwrap().to_owned(),
    })
}
