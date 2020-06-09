use std::path::{PathBuf, Path};
use std::io::prelude::*;
use std::io::{Result as IOResult};
use std::fs;
use std::fs::File;

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