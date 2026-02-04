pub mod forest_dot_writer;
pub mod instance_reader;
pub mod solution_reader;

#[cfg(test)]
pub(crate) mod tests {
    use std::ffi::OsStr;
    use std::path::{Path, PathBuf};

    pub(crate) fn test_instances_directory(name: impl AsRef<Path>) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("testcases")
            .join(name)
    }

    pub(crate) fn test_instances(name: &str) -> Vec<(PathBuf, Option<PathBuf>)> {
        let dir = test_instances_directory(name);

        let mut result = Vec::new();

        for f in dir.read_dir().unwrap() {
            if let Ok(file) = f {
                let input_path = file.path();
                if input_path.extension() != Some(OsStr::new("in")) {
                    continue;
                }

                let output_path = {
                    let mut output_path = input_path.clone();
                    output_path.set_extension("out");

                    output_path.exists().then(|| output_path)
                };

                result.push((input_path, output_path));
            }
        }

        assert!(result.len() > 0);

        result
    }
}
