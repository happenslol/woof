use crate::context::Diagnostics;
use crate::errors::WoofError;
use crate::parse::{Locale, Module, build_flat_module, build_namespaced_module};
use crate::sanitize::is_valid_identifier;
use std::collections::HashMap;
use std::path::Path;
use std::{env, fs};
use toml::Value;

#[derive(Debug, PartialEq)]
pub enum FileMode {
  Flat,
  Namespaced,
}

#[derive(Debug)]
pub struct ParsedFile {
  /// The path of the file, either relative to the current working directory or (if outside of it)
  /// relative to the input directory
  pub normalized_path: String,
  pub contents: Value,
}

#[derive(Debug)]
pub struct NamespacedFile {
  pub namespace: String,
  pub file: ParsedFile,
}

/// Determines the file mode by examining the files in the directory
fn detect_file_mode(dir: &Path) -> Result<FileMode, WoofError> {
  let entries = fs::read_dir(dir)?;
  let toml_files = entries
    .filter_map(|e| e.ok())
    .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("toml"))
    .collect::<Vec<_>>();

  let mut has_flat = false;
  let mut has_namespaced = false;

  for entry in toml_files {
    let path = entry.path();
    let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
      continue;
    };

    if stem.contains('.') {
      has_namespaced = true;
    } else {
      has_flat = true;
    }

    // Error if both modes are detected
    if has_flat && has_namespaced {
      return Err(WoofError::MixedFileModes);
    }
  }

  if has_namespaced {
    Ok(FileMode::Namespaced)
  } else {
    Ok(FileMode::Flat)
  }
}

/// Collects locale files from a directory (flat mode)
fn collect_flat(input_dir: &Path) -> Result<HashMap<Locale, ParsedFile>, WoofError> {
  let cwd = env::current_dir().map_err(WoofError::InvalidCwd)?;
  let mut result = HashMap::new();

  if !input_dir.is_dir() {
    return Err(WoofError::InvalidInputDirectory(
      input_dir.display().to_string(),
    ));
  }

  let entries = fs::read_dir(input_dir)?;
  let toml_files = entries
    .filter_map(|e| e.ok())
    .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("toml"));

  for entry in toml_files {
    let path = entry.path();
    let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
      // TODO: Log this
      continue;
    };

    let normalized_path = normalize_path(&path, &cwd, input_dir);

    let locale = Locale(stem.to_string());
    let contents = fs::read_to_string(&path)?;
    let contents = toml::from_str(&contents).map_err(|err| {
      let filename = path
        .file_name()
        .map(|s| s.to_string_lossy())
        .unwrap_or_default()
        .to_string();

      WoofError::Toml(filename, err)
    })?;

    let file = ParsedFile {
      normalized_path,
      contents,
    };

    result.insert(locale, file);
  }

  Ok(result)
}

/// Collects namespaced files from a directory
fn collect_namespaced(input_dir: &Path) -> Result<HashMap<Locale, NamespacedFile>, WoofError> {
  let cwd = env::current_dir().map_err(WoofError::InvalidCwd)?;
  let mut result = HashMap::new();

  if !input_dir.is_dir() {
    return Err(WoofError::InvalidInputDirectory(
      input_dir.display().to_string(),
    ));
  }

  let entries = fs::read_dir(input_dir)?;
  let toml_files = entries
    .filter_map(|e| e.ok())
    .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("toml"));

  for entry in toml_files {
    let path = entry.path();
    let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
      // TODO: Log this
      continue;
    };

    let normalized_path = normalize_path(&path, &cwd, input_dir);

    // Parse namespace.locale format
    let parts: Vec<&str> = stem.split('.').collect();
    if parts.len() != 2 || parts[0].is_empty() || !is_valid_identifier(parts[0]) {
      return Err(WoofError::InvalidFileName(
        path
          .file_name()
          .unwrap_or_default()
          .to_string_lossy()
          .to_string(),
      ));
    }

    let namespace = parts[0].to_string();
    let locale = Locale(parts[1].to_string());

    let contents = fs::read_to_string(&path)?;
    let contents: Value = toml::from_str(&contents).map_err(|err| {
      let filename = path
        .file_name()
        .map(|s| s.to_string_lossy())
        .unwrap_or_default()
        .to_string();
      WoofError::Toml(filename, err)
    })?;

    let file = ParsedFile {
      normalized_path,
      contents,
    };

    result.insert(locale, NamespacedFile { namespace, file });
  }

  Ok(result)
}

pub struct ModuleBuildResult {
  pub module: Module,
  pub diagnostics: Diagnostics,
  pub locales: Vec<Locale>,
}

/// Collects and builds modules from translation files, supporting both flat and namespaced modes
pub fn collect_and_build_modules(dir: &str) -> Result<ModuleBuildResult, WoofError> {
  let dir = Path::new(dir);
  let mode = detect_file_mode(dir)?;

  match mode {
    FileMode::Flat => {
      let files = collect_flat(dir)?;
      let locales = files.keys().cloned().collect::<Vec<_>>();
      let (module, diagnostics) = build_flat_module(files)?;

      Ok(ModuleBuildResult {
        module,
        diagnostics,
        locales,
      })
    }
    FileMode::Namespaced => {
      let files = collect_namespaced(dir)?;
      let locales = files.keys().cloned().collect::<Vec<_>>();
      let mut namespaces = HashMap::new();

      for (locale, file) in files {
        namespaces
          .entry(file.namespace)
          .or_insert_with(HashMap::new)
          .insert(locale, file.file);
      }

      let (module, diagnostics) = build_namespaced_module(namespaces)?;

      Ok(ModuleBuildResult {
        module,
        diagnostics,
        locales,
      })
    }
  }
}

fn normalize_path(path: &Path, cwd: &Path, input_dir: &Path) -> String {
  if let Ok(path) = path.strip_prefix(cwd) {
    return path.display().to_string();
  }

  if let Ok(path) = path.strip_prefix(input_dir) {
    return path.display().to_string();
  }

  path.display().to_string()
}
