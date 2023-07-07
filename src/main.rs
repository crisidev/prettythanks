use camino::{Utf8Path, Utf8PathBuf};
use std::{env, fs, time::Instant};

type BoxError = Box<dyn std::error::Error>;
type Result<T> = std::result::Result<T, BoxError>;

/// pretty-thanks - a frontend to dtolnay/prettyplease library.
#[derive(argh::FromArgs)]
struct Args {
    /// path to recursively format (default to the current directory).
    #[argh(option, short = 'p')]
    path: Option<String>,
    /// print out information about what is being formatted.
    #[argh(switch, short = 'v')]
    verbose: bool,
}

struct PrettyThanks {
    path: Utf8PathBuf,
}

/// I know, this is ugly, but I want to keep dependencies to the minimum possible.
static mut VERBOSE: bool = false;

/// Only print if the `VERBOSE` flag is set.
macro_rules! vprintln {
    ($($arg:tt)*) => (
        if unsafe { VERBOSE } {
            ::std::println!($($arg)*);
        }
    )
}

impl PrettyThanks {
    fn new(path: Option<&str>) -> Result<Self> {
        let path = match path.as_ref() {
            Some(path) => path.into(),
            None => env::current_dir()?.canonicalize()?.try_into()?,
        };
        Ok(PrettyThanks { path })
    }

    fn run(&self) -> Result<()> {
        let start = Instant::now();
        if self.path.extension() == Some("rs") && (self.path.is_file() || self.path.is_symlink()) {
            let (original, formatted) = self.format_file(&self.path)?;
            vprintln!(
                "formatting completed, original size: {} bytes, formatted size: {} bytes, time: {} ms",
                original,
                formatted,
                start.elapsed().as_millis()
            );
            Ok(())
        } else if self.path.is_dir() {
            let (original, formatted) = self.format_directory(&self.path)?;
            vprintln!(
                "formatting completed, original size: {} bytes, formatted size: {} bytes, time: {} ms",
                original,
                formatted,
                start.elapsed().as_millis()
            );
            Ok(())
        } else {
            Err(format!("path {} is not a file, symlink or directory", self.path).into())
        }
    }

    fn format_file(&self, path: &Utf8Path) -> Result<(usize, usize)> {
        let start = Instant::now();
        let original =
            fs::read_to_string(path).map_err(|err| format!("failed to read file {path}: {err}"))?;
        let ast = syn::parse_file(&original)
            .map_err(|err| format!("failed to parse file {path}: {err}"))?;
        let formatted = prettyplease::unparse(&ast);
        vprintln!(
            "formatting file {}, original size {} bytes, formatted size {} bytes, time: {} ms",
            path,
            original.len(),
            formatted.len(),
            start.elapsed().as_millis()
        );
        fs::write(path, &formatted).map_err(|err| format!("failed to write file {path}: {err}"))?;
        Ok((original.len(), formatted.len()))
    }

    fn format_directory(&self, path: &Utf8Path) -> Result<(usize, usize)> {
        let (mut original, mut formatted) = (0usize, 0usize);
        let mut errors = Vec::new();
        for entry in path.read_dir_utf8()? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            if entry.path().extension() == Some("rs")
                && (file_type.is_file() || file_type.is_symlink())
            {
                match self.format_file(entry.path()) {
                    Ok((o, f)) => {
                        original += o;
                        formatted += f;
                    }
                    Err(e) => errors.push((entry.path().to_string(), e)),
                }
            } else if file_type.is_dir() || file_type.is_symlink() {
                let (o, f) = self.format_directory(entry.path())?;
                original += o;
                formatted += f;
            }
        }
        if errors.is_empty() {
            Ok((original, formatted))
        } else {
            Err(errors
                .into_iter()
                .map(|entry| format!("error: {}: {}", entry.0, entry.1))
                .collect::<Vec<String>>()
                .join("\n")
                .into())
        }
    }
}

fn main() -> Result<()> {
    let args: Args = argh::from_env();
    unsafe { VERBOSE = args.verbose };
    let pretty_thanks = PrettyThanks::new(args.path.as_deref())?;
    pretty_thanks.run()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;

    #[test]
    fn can_format() {
        let temp_file = temp_dir().join("prettythanks.rs");
        fs::copy("fixtures/input.rs", &temp_file).unwrap();
        let thanks = PrettyThanks::new(temp_file.to_str()).unwrap();
        assert!(thanks.run().is_ok());
    }
}
