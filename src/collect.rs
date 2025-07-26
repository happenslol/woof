use crate::context::Diagnostics;
use crate::errors::WoofError;
use crate::parse::{Locale, Module, build_flat_module, build_namespaced_module};
use crate::sanitize::is_valid_identifier;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use toml::Value;

#[derive(Debug, PartialEq)]
pub enum FileMode {
  Flat,
  Namespaced,
}

#[derive(Debug)]
pub struct NamespacedFile {
  pub namespace: String,
  pub locale: Locale,
  pub content: Value,
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
fn collect_flat<P: AsRef<Path>>(dir: P) -> Result<HashMap<Locale, Value>, WoofError> {
  let dir = dir.as_ref();
  let mut result = HashMap::new();

  if !dir.is_dir() {
    return Err(WoofError::InvalidInputDirectory(dir.display().to_string()));
  }

  let entries = fs::read_dir(dir)?;
  let toml_files = entries
    .filter_map(|e| e.ok())
    .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("toml"));

  for entry in toml_files {
    let path = entry.path();
    let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
      continue;
    };

    let contents = fs::read_to_string(&path)?;
    let locale = Locale(stem.to_string());
    let parsed = toml::from_str(&contents).map_err(|err| {
      let filename = path
        .file_name()
        .map(|s| s.to_string_lossy())
        .unwrap_or_default()
        .to_string();

      WoofError::Toml(filename, err)
    })?;

    result.insert(locale, parsed);
  }

  Ok(result)
}

/// Collects namespaced files from a directory
fn collect_namespaced<P: AsRef<Path>>(dir: P) -> Result<Vec<NamespacedFile>, WoofError> {
  let dir = dir.as_ref();
  let mut result = Vec::new();

  if !dir.is_dir() {
    return Err(WoofError::InvalidInputDirectory(dir.display().to_string()));
  }

  let entries = fs::read_dir(dir)?;
  let toml_files = entries
    .filter_map(|e| e.ok())
    .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("toml"));

  for entry in toml_files {
    let path = entry.path();
    let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
      continue;
    };

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
    let content: Value = toml::from_str(&contents).map_err(|err| {
      let filename = path
        .file_name()
        .map(|s| s.to_string_lossy())
        .unwrap_or_default()
        .to_string();
      WoofError::Toml(filename, err)
    })?;

    result.push(NamespacedFile {
      namespace,
      locale,
      content,
    });
  }

  Ok(result)
}

/// Collects and builds modules from translation files, supporting both flat and namespaced modes
pub fn collect_and_build_modules<P: AsRef<Path>>(
  dir: P,
) -> Result<(Module, Diagnostics), WoofError> {
  let dir = dir.as_ref();
  let mode = detect_file_mode(dir)?;

  match mode {
    FileMode::Flat => {
      let locales = collect_flat(dir)?;
      build_flat_module(locales)
    }
    FileMode::Namespaced => {
      let files = collect_namespaced(dir)?;

      let mut namespaces = HashMap::new();

      for file in files {
        namespaces
          .entry(file.namespace)
          .or_insert_with(HashMap::new)
          .insert(file.locale, file.content);
      }

      build_namespaced_module(namespaces)
    }
  }
}
