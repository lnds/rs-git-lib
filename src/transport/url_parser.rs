use std::fmt::Formatter;
use std::io::{Error, ErrorKind, Result as IOResult};
use url::{ParseError, Url};

#[derive(Debug, PartialEq)]
pub(crate) enum UrlType {
    LOCAL(String, String),
    FILE(String, String),
    GIT(Url, String),
    HTTP(Url, String),
    SSH(Url, String),
}

#[derive(Debug, PartialEq)]
pub enum UrlError {
    UrlParseError(ParseError),
    BadScheme,
    Empty,
    NoServer,
    NoPath,
    InvalidPath,
}

impl std::error::Error for UrlError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            UrlError::UrlParseError(e) => Some(e),
            UrlError::BadScheme => None,
            UrlError::Empty => None,
            UrlError::NoServer => None,
            UrlError::NoPath => None,
            UrlError::InvalidPath => None,
        }
    }
}

impl std::fmt::Display for UrlError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            UrlError::UrlParseError(e) => e.fmt(f)?,
            UrlError::BadScheme => f.write_str("bad scheme")?,
            UrlError::Empty => f.write_str("empty")?,
            UrlError::NoServer => f.write_str("no server")?,
            UrlError::NoPath => f.write_str("no path")?,
            UrlError::InvalidPath => f.write_str("invalid path")?,
        }
        Ok(())
    }
}

pub(crate) fn parse(url: &str, dir: Option<String>) -> IOResult<UrlType> {
    match Url::parse(url) {
        Ok(url) => match url.scheme() {
            "git" => parse_git(&url, dir),
            "http" | "https" => parse_http(&url, dir),
            "ssh" => parse_ssh(&url, dir),
            "file" => parse_file(&url, dir),
            _ => Err(Error::new(ErrorKind::Other, UrlError::BadScheme)),
        },
        Err(ParseError::EmptyHost) => Err(Error::new(ErrorKind::Other, UrlError::NoServer)),
        Err(ParseError::RelativeUrlWithoutBase) => parse_local(url, dir),
        Err(e) => Err(Error::new(ErrorKind::Other, UrlError::UrlParseError(e))),
    }
}

fn parse_git(url: &Url, dir: Option<String>) -> Result<UrlType, Error> {
    if !url.has_host() {
        Err(Error::new(ErrorKind::Other, UrlError::NoServer))
    } else if url.path().is_empty() {
        Err(Error::new(ErrorKind::Other, UrlError::NoPath))
    } else {
        Ok(UrlType::GIT(url.clone(), det_output_dir(url.path(), dir)))
    }
}

fn parse_http(url: &Url, dir: Option<String>) -> Result<UrlType, Error> {
    if !url.has_host() {
        Err(Error::new(ErrorKind::Other, UrlError::NoServer))
    } else if url.path().is_empty() || url.path() == "/" {
        Err(Error::new(ErrorKind::Other, UrlError::NoPath))
    } else {
        Ok(UrlType::HTTP(url.clone(), det_output_dir(url.path(), dir)))
    }
}

fn parse_ssh(url: &Url, dir: Option<String>) -> Result<UrlType, Error> {
    if !url.has_host() {
        Err(Error::new(ErrorKind::Other, UrlError::NoServer))
    } else if url.path().is_empty() || url.path() == "/" {
        Err(Error::new(ErrorKind::Other, UrlError::NoPath))
    } else {
        Ok(UrlType::SSH(url.clone(), det_output_dir(url.path(), dir)))
    }
}

fn parse_file(url: &Url, dir: Option<String>) -> Result<UrlType, Error> {
    if url.path().is_empty() || url.path() == "/" {
        Err(Error::new(ErrorKind::Other, UrlError::InvalidPath))
    } else {
        Ok(UrlType::FILE(
            url.path().to_string(),
            det_output_dir(url.path(), dir),
        ))
    }
}

fn parse_local(url: &str, dir: Option<String>) -> Result<UrlType, Error> {
    if url.is_empty() {
        return Err(Error::new(ErrorKind::Other, UrlError::Empty));
    }
    if url.contains('@') {
        // alternate SSH
        let pos_a = url.find('@').unwrap_or(0);
        let pos_b = url.find(':').unwrap_or(0);
        let pos_c = url.rfind(':').unwrap_or(0);
        if pos_b == pos_c
            && pos_b > pos_a
            && pos_b < url.len() - 1
            && &url[pos_b..pos_b + 2usize] != ":/"
        {
            let s = format!("ssh://{}/{}", &url[0..pos_b], &url[pos_b + 1..]);
            return parse(&s, dir);
        }
        return parse(&format!("ssh://{}", url), dir);
    }
    Ok(UrlType::LOCAL(url.to_string(), det_output_dir(url, dir)))
}

fn det_output_dir(remote_path: &str, dir: Option<String>) -> String {
    let result = match remote_path.rfind('/') {
        Some(p) => dir.unwrap_or_else(|| remote_path[p + 1..].to_string()),
        None => dir.unwrap_or_else(|| remote_path.to_string()),
    };
    result.trim_end_matches(".git").to_string()
}

#[cfg(test)]
mod tests {

    use super::parse;
    use super::UrlType;

    #[test]
    fn test_parse() {
        assert!(
            parse("git://a.b/repo.git", None).is_ok(),
            "bad parse of git"
        );
        assert!(
            parse("http://a.b/repo.git", None).is_ok(),
            "bad parse of http"
        );
        assert!(
            parse("https://a.b/repo.git", None).is_ok(),
            "bad parse of https"
        );
        assert!(
            parse("ssh://a.b/repo.git", None).is_ok(),
            "bad parse of ssh"
        );
        assert!(
            parse("file:///a.b/repo.git", None).is_ok(),
            "bad parse of file"
        );
        assert!(
            parse("warara://a.b/repo.git", None).is_err(),
            "bad parse of bad proto"
        );
        assert!(parse("/a/b/repo.git", None).is_ok(), "bad parse of file");
        assert!(parse("", None).is_err(), "bad parse of empty")
    }

    #[test]
    fn test_git() {
        assert!(parse("git://", None).is_err());
        assert!(parse("git://server", None).is_err());
        let res = parse("git://server/path", None);
        assert!(res.is_ok());
        if let Some(UrlType::GIT(_url, path)) = res.ok() {
            assert_eq!(path, "path");
        } else {
            panic!("failed git parse");
        }
    }

    #[test]
    fn test_http() {
        assert!(parse("http://", None).is_err());
        assert!(parse("https://", None).is_err());
        assert!(parse("http://domain", None).is_err());
        assert!(parse("https://domain", None).is_err());
        assert!(parse("http://domain.tld", None).is_err());
        assert!(parse("https://domain.tld", None).is_err());
        assert!(parse("http://domain.tld/", None).is_err());
        assert!(parse("https://domain.tld/", None).is_err(),);
        assert!(parse("http://domain.tld", None).is_err());
        assert!(parse("https://domain.tld", None).is_err());
        let res = parse("https://server/path", None);
        if let Some(UrlType::HTTP(_url, path)) = res.ok() {
            assert_eq!(path, "path");
        } else {
            panic!("failed http parse");
        }
    }

    #[test]
    fn test_ssh() {
        assert!(parse("ssh://", None).is_err());
        assert!(parse("git+ssh://", None).is_err());
        assert!(parse("ssh://domain", None).is_err());
        assert!(parse("user@host", None).is_err());
        assert!(parse("user@host:/", None).is_err());
        assert!(parse("login@server.com:12345/~/repository.git", None).is_ok());
        let res = parse("ssh://login@server.com:12345/~/repository.git", None);
        if let Some(UrlType::SSH(_url, path)) = res.ok() {
            assert_eq!(path, "repository");
        } else {
            panic!("failed ssh parse");
        }

        let res = parse("git@server.com:user/repository.git", None);
        if let Some(UrlType::SSH(_url, path)) = res.ok() {
            assert_eq!(path, "repository");
        } else {
            panic!("failed ssh parse {:?}");
        }
    }

    #[test]
    fn test_file() {
        assert!(parse("file://", None).is_err());
        assert!(parse("file://path", None).is_err());
        assert!(parse("file://domain", None).is_err());
        let res = parse("file:///user/work/repository.git", None);
        if let Some(UrlType::FILE(_url, path)) = res.ok() {
            assert_eq!(path, "repository");
        } else {
            panic!("failed ssh parse");
        }
    }

    #[test]
    fn test_local() {
        assert!(parse("/home/user/repo.git", None).is_ok());
    }
}
