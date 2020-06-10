use crate::utils::is_sha;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::io::Result as IOResult;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Ref {
    pub id: String,
    pub name: String,
}

pub type Refs = Vec<Ref>;

pub(crate) fn create_refs(repo: &str, refs: &[Ref]) -> IOResult<()> {
    let (tags, branches): (Vec<_>, Vec<_>) = refs
        .iter()
        .filter(|r| !r.name.ends_with("^{}"))
        .partition(|r| r.name.starts_with("refs/tags"));

    write_refs(repo, "refs/remotes/origin", &branches)?;
    write_refs(repo, "refs/tags", &tags)?;
    Ok(())
}

pub(crate) fn update_head(repo: &str, refs: &[Ref]) -> IOResult<()> {
    if let Some(head) = refs.iter().find(|r| r.name == "HEAD") {
        let sha1 = &head.id;
        let true_ref = refs.iter().find(|r| r.name != "HEAD" && r.id == *sha1);
        let dir = true_ref.map_or("refs/heads/master", |r| &r.name[..]);
        create_ref(repo, dir, &sha1)?;
        create_sym_ref(repo, "HEAD", dir)?;
    }
    Ok(())
}

fn write_refs(repo: &str, parent_path: &str, refs: &[&Ref]) -> IOResult<()> {
    let mut path = PathBuf::new();
    path.push(parent_path);

    for r in refs {
        let mut full_path = path.clone();
        let simple_name = Path::new(&r.name).file_name().unwrap();
        full_path.push(&simple_name);
        create_ref(repo, full_path.to_str().unwrap(), &r.id)?;
    }
    Ok(())
}

fn create_ref(repo: &str, path: &str, id: &str) -> IOResult<()> {
    let mut full_path = PathBuf::new();
    full_path.push(repo);
    full_path.push(".git");
    full_path.push(path);
    fs::create_dir_all(full_path.parent().unwrap())?;
    let mut file = File::create(full_path)?;
    file.write_fmt(format_args!("{}\n", id))?;
    Ok(())
}

///
/// Creates a symbolic ref in the given repository.
///
fn create_sym_ref(repo: &str, name: &str, the_ref: &str) -> IOResult<()> {
    let mut path = PathBuf::new();
    path.push(repo);
    path.push(".git");
    path.push(name);
    let mut file = File::create(path)?;
    file.write_fmt(format_args!("ref: {}\n", the_ref))?;
    Ok(())
}

pub fn resolve_ref(repo: &str, name: &str) -> IOResult<String> {
    // Check if the name is already a sha.
    let trimmed = name.trim();
    if is_sha(trimmed) {
        Ok(trimmed.to_owned())
    } else {
        read_sym_ref(repo, trimmed)
    }
}

fn read_sym_ref(repo: &str, name: &str) -> IOResult<String> {
    // Read the symbolic ref directly and parse the actual ref out
    let mut path = PathBuf::new();
    path.push(repo);
    path.push(".git");

    if name != "HEAD" {
        if !name.contains('/') {
            path.push("refs/heads");
        } else if !name.starts_with("refs/") {
            path.push("refs/remotes");
        }
    }
    path.push(name);

    // Read the actual ref out
    let mut contents = String::new();
    let mut file = File::open(path)?;
    file.read_to_string(&mut contents)?;

    if contents.starts_with("ref: ") {
        let the_ref = contents.split("ref: ").nth(1).unwrap().trim();
        resolve_ref(repo, &the_ref)
    } else {
        Ok(contents.trim().to_owned())
    }
}
