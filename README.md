# rs-git-lib

Rust Native Git Library

## Usage

    use rs_git_lib::Repo;
    let repo = Repo::clone_from("https://github.com/lnds/rs-git-lib.git", Some("/tmp/rs-git".to_string())).unwrap();
    
    let commits = repo.commits();
    assert_eq!(commits[commits.len()].as_commit().unwrap().get_message(), "Initial commit")


    
## Note

This work started from the ideas and code in Rgit project by @cwbriones:

    https://github.com/cwbriones/rgit
    
