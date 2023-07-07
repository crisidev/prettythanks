use camino::{Utf8Path, Utf8PathBuf};
use std::{
    env, fs,
    io::{Error, ErrorKind},
};

type BoxError = Box<dyn std::error::Error>;
type Result<T> = std::result::Result<T, BoxError>;

/// pretty-thanks - a frontend to dtolnay/pretty-please library.
#[derive(argh::FromArgs)]
struct Args {
    /// path to recursively format (default to the current directory)
    #[argh(option)]
    path: Option<String>,
}

struct PrettyThanks {
    path: Utf8PathBuf,
}

impl PrettyThanks {
    fn new(path: Option<String>) -> Result<Self> {
        let path = match path.as_ref() {
            Some(path) => path.into(),
            None => env::current_dir()?.canonicalize()?.try_into()?,
        };
        Ok(PrettyThanks { path })
    }

    fn run(&self) -> Result<()> {
        if self.path.extension() == Some("rs") && (self.path.is_file() || self.path.is_symlink()) {
            let (original, formatted) = self.format_file(&self.path)?;
            println!("format completed, original size: {original} bytes, formatted size: {formatted} bytes");
            Ok(())
        } else if self.path.is_dir() {
            let (original, formatted) = self.format_directory(&self.path)?;
            println!("format completed, original size: {original} bytes, formatted size: {formatted} bytes");
            Ok(())
        } else {
            Err(Box::new(Error::new(
                ErrorKind::Other,
                format!("path {} is not a file, symlink or directory", self.path),
            )))
        }
    }

    fn format_file(&self, path: &Utf8Path) -> Result<(usize, usize)> {
        let original =
            fs::read_to_string(path).map_err(|err| format!("failed to read file {path}: {err}"))?;
        let ast = syn::parse_file(&original)
            .map_err(|err| format!("failed to parse file {path}: {err}"))?;
        let formatted = prettyplease::unparse(&ast);
        println!(
            "formatting file {}, original size {} bytes, formatted size {} bytes",
            path,
            original.len(),
            formatted.len()
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
    let pretty_thanks = PrettyThanks::new(args.path)?;
    pretty_thanks.run()
}
