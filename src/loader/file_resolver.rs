use crate::{Ctx, Error, Resolver, Result};
use relative_path::{RelativePath, RelativePathBuf};

/// The file module resolver
///
/// This resolver can be used as the nested backing resolver in user-defined resolvers.
#[derive(Debug)]
pub struct FileResolver {
    paths: Vec<RelativePathBuf>,
    patterns: Vec<String>,
}

impl FileResolver {
    /// Add search path for modules
    pub fn add_path<P: Into<RelativePathBuf>>(&mut self, path: P) -> &mut Self {
        self.paths.push(path.into());
        self
    }

    /// Add module file pattern
    pub fn add_pattern<P: Into<String>>(&mut self, pattern: P) -> &mut Self {
        self.patterns.push(pattern.into());
        self
    }

    /// Add support for native modules
    pub fn add_native(&mut self) -> &mut Self {
        #[cfg(target_family = "windows")]
        self.add_pattern("{}.dll");

        #[cfg(target_vendor = "apple")]
        self.add_pattern("{}.dylib").add_pattern("lib{}.dylib");

        #[cfg(all(target_family = "unix"))]
        self.add_pattern("{}.so").add_pattern("lib{}.so");

        self
    }

    /// Build resolver
    pub fn build(&self) -> Self {
        Self {
            paths: self.paths.clone(),
            patterns: self.patterns.clone(),
        }
    }

    fn try_patterns(&self, path: &RelativePath) -> Option<RelativePathBuf> {
        if let Some(extension) = &path.extension() {
            if !is_file(path) {
                return None;
            }
            // check for known extensions
            self.patterns
                .iter()
                .find(|pattern| {
                    let path = RelativePath::new(pattern);
                    if let Some(known_extension) = &path.extension() {
                        known_extension == extension
                    } else {
                        false
                    }
                })
                .map(|_| path.to_relative_path_buf())
        } else {
            // try with known patterns
            self.patterns
                .iter()
                .filter_map(|pattern| {
                    let name = pattern.replace("{}", path.file_name()?);
                    let file = path.with_file_name(&name);
                    if is_file(&file) {
                        Some(file)
                    } else {
                        None
                    }
                })
                .next()
        }
    }
}

impl Default for FileResolver {
    fn default() -> Self {
        Self {
            paths: vec![],
            patterns: vec!["{}.js".into()],
        }
    }
}

impl Resolver for FileResolver {
    fn resolve<'js>(&mut self, _ctx: Ctx<'js>, base: &str, name: &str) -> Result<String> {
        let path = if !name.starts_with('.') {
            self.paths
                .iter()
                .filter_map(|path| {
                    let path = path.join_normalized(name);
                    self.try_patterns(&path)
                })
                .next()
        } else {
            let path = RelativePath::new(base);
            let path = if let Some(dir) = path.parent() {
                dir.join_normalized(name)
            } else {
                name.into()
            };
            self.try_patterns(&path)
        }
        .ok_or_else(|| Error::resolving::<_, _, &str>(base, name, None))?;

        Ok(path.to_string())
    }
}

fn is_file<P: AsRef<RelativePath>>(path: P) -> bool {
    path.as_ref().to_path(".").is_file()
}
