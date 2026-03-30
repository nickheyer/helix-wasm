use std::{
    fmt,
    path::{Path, PathBuf},
    sync::Arc,
};

/// A generic pointer to a file location.
///
/// Currently this type only supports paths to local files.
///
/// Cloning this type is cheap: the internal representation uses an Arc.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Uri {
    File(Arc<Path>),
}

impl Uri {
    // This clippy allow mirrors url::Url::from_file_path
    #[allow(clippy::result_unit_err)]
    pub fn to_url(&self) -> Result<url::Url, ()> {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            Uri::File(path) => url::Url::from_file_path(path),
            #[cfg(target_arch = "wasm32")]
            Uri::File(path) => {
                let path_str = path.to_string_lossy();
                url::Url::parse(&format!("file://{path_str}")).map_err(|_| ())
            }
        }
    }

    pub fn as_path(&self) -> Option<&Path> {
        match self {
            Self::File(path) => Some(path),
        }
    }
}

impl From<PathBuf> for Uri {
    fn from(path: PathBuf) -> Self {
        Self::File(path.into())
    }
}

impl fmt::Display for Uri {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::File(path) => write!(f, "{}", path.display()),
        }
    }
}

#[derive(Debug)]
pub struct UrlConversionError {
    source: url::Url,
    kind: UrlConversionErrorKind,
}

#[derive(Debug)]
pub enum UrlConversionErrorKind {
    UnsupportedScheme,
    UnableToConvert,
}

impl fmt::Display for UrlConversionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            UrlConversionErrorKind::UnsupportedScheme => {
                write!(
                    f,
                    "unsupported scheme '{}' in URL {}",
                    self.source.scheme(),
                    self.source
                )
            }
            UrlConversionErrorKind::UnableToConvert => {
                write!(f, "unable to convert URL to file path: {}", self.source)
            }
        }
    }
}

impl std::error::Error for UrlConversionError {}

fn convert_url_to_uri(url: &url::Url) -> Result<Uri, UrlConversionErrorKind> {
    if url.scheme() == "file" {
        #[cfg(not(target_arch = "wasm32"))]
        {
            url.to_file_path()
                .map(|path| Uri::File(helix_stdx::path::normalize(path).into()))
                .map_err(|_| UrlConversionErrorKind::UnableToConvert)
        }
        #[cfg(target_arch = "wasm32")]
        {
            let path = PathBuf::from(url.path());
            Ok(Uri::File(helix_stdx::path::normalize(path).into()))
        }
    } else {
        Err(UrlConversionErrorKind::UnsupportedScheme)
    }
}

impl TryFrom<url::Url> for Uri {
    type Error = UrlConversionError;

    fn try_from(url: url::Url) -> Result<Self, Self::Error> {
        convert_url_to_uri(&url).map_err(|kind| Self::Error { source: url, kind })
    }
}

impl TryFrom<&url::Url> for Uri {
    type Error = UrlConversionError;

    fn try_from(url: &url::Url) -> Result<Self, Self::Error> {
        convert_url_to_uri(url).map_err(|kind| Self::Error {
            source: url.clone(),
            kind,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use url::Url;

    #[test]
    fn unknown_scheme() {
        let url = Url::parse("csharp:/metadata/foo/bar/Baz.cs").unwrap();
        assert!(matches!(
            Uri::try_from(url),
            Err(UrlConversionError {
                kind: UrlConversionErrorKind::UnsupportedScheme,
                ..
            })
        ));
    }
}
