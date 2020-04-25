use std::io;
use std::path::{Component, Path, PathBuf};

pub fn absolutize<P, Q>(relative_to: P, path: Q) -> PathBuf
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let path = relative_to.as_ref().join(path);

    let (relative_to, path) = {
        let components: Vec<_> = path.components().collect();
        let separator = components
            .iter()
            .enumerate()
            .find(|(_, c)| c == &&Component::CurDir || c == &&Component::ParentDir);

        if let Some((index, _)) = separator {
            let (absolute, relative) = components.split_at(index);
            let absolute: PathBuf = absolute.iter().collect();
            let relative: PathBuf = relative.iter().collect();

            (absolute, relative)
        } else {
            (relative_to.as_ref().to_path_buf(), path)
        }
    };

    let path = if path.is_relative() {
        let mut result = relative_to;
        path.components().for_each(|component| match component {
            Component::ParentDir => {
                result.pop();
            }
            Component::Normal(normal) => result.push(normal),
            _ => {}
        });

        result
    } else {
        path
    };

    dunce::simplified(&path).to_path_buf()
}

pub fn canonicalize<P, Q>(relative_to: P, path: Q) -> io::Result<PathBuf>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let absolutized = absolutize(&relative_to, path);
    let path = match std::fs::read_link(&absolutized) {
        Ok(resolved) => {
            let parent = absolutized.parent().unwrap_or(&absolutized);
            absolutize(parent, resolved)
        }

        Err(e) => {
            if absolutized.exists() {
                absolutized
            } else {
                return Err(e);
            }
        }
    };

    Ok(dunce::simplified(&path).to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn absolutize_two_dots() {
        let relative_to = Path::new("/foo/bar");
        let path = Path::new("..");

        assert_eq!(
            PathBuf::from("/foo"), // missing path
            absolutize(relative_to, path)
        );
    }

    #[test]
    fn canonicalize_should_succeed() -> io::Result<()> {
        let relative_to = Path::new("/foo/bar");
        let path = Path::new("../..");

        assert_eq!(
            PathBuf::from("/"), // existing path
            canonicalize(relative_to, path)?,
        );

        Ok(())
    }

    #[test]
    fn canonicalize_should_fail() {
        let relative_to = Path::new("/foo/bar/baz"); // '/foo' is missing
        let path = Path::new("../..");

        assert!(canonicalize(relative_to, path).is_err());
    }
}
