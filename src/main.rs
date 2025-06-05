use clap::Parser;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
use toml::{Table, Value};

type Result<T> = std::result::Result<T, WoofError>;

#[derive(Error, Debug)]
pub enum WoofError {
  #[error("IO error: {0}")]
  Io(#[from] std::io::Error),

  #[error("Path `{0}` is not a directory")]
  InvalidInputDirectory(String),

  #[error("Output file already exists at `{0}`")]
  OutputFileExists(PathBuf),

  #[error("Root of TOML file is not a table")]
  RootNotTable,

  #[error("Invalid TOML: {0}")]
  Toml(#[from] toml::de::Error),

  #[error("Unsupported value type `{typename}` at path `{path}`")]
  UnsupportedValueType { typename: String, path: String },

  #[error("Invalid interpolation format in string: {0}")]
  InvalidInterpolation(String),

  #[error("Interpolation type mismatch between locales")]
  InterpolationTypeMismatch,
}

#[derive(Debug)]
struct Interpolation {
  typename: Option<String>,
  ranges: HashMap<String, (usize, usize)>,
}

#[derive(Debug)]
struct Message {
  value: HashMap<String, String>,
  interpolations: HashMap<String, Interpolation>,
}

fn collect_locales<P: AsRef<Path>>(dir: P) -> Result<HashMap<String, Value>> {
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

    let stem = stem.to_string();
    let contents = fs::read_to_string(path)?;
    result.insert(stem.to_string(), toml::from_str(&contents)?);
  }

  Ok(result)
}

#[derive(Debug)]
struct Module {
  messages: BTreeMap<String, Message>,
  modules: BTreeMap<String, Module>,
}

fn build_modules(locales: HashMap<String, Value>) -> Result<Module> {
  let mut messages = BTreeMap::new();
  let mut modules = BTreeMap::new();

  for (locale, value) in locales {
    let Value::Table(root) = value else {
      return Err(WoofError::RootNotTable);
    };

    build_module(&locale, root, &mut messages, &mut modules)?;
  }

  Ok(Module { messages, modules })
}

fn build_module(
  locale: &str,
  table: Table,
  messages: &mut BTreeMap<String, Message>,
  modules: &mut BTreeMap<String, Module>,
) -> Result<()> {
  for (key, value) in table {
    match value {
      Value::String(s) => {
        let interpolations = parse_interpolations(&s)?;
        let message = messages.entry(key.clone()).or_insert_with(|| Message {
          value: HashMap::new(),
          interpolations: HashMap::new(),
        });

        message.value.insert(locale.to_string(), s.clone());
        for interpolation in interpolations {
          let entry = message
            .interpolations
            .entry(interpolation.name)
            .or_insert_with(|| Interpolation {
              typename: interpolation.typename.clone(),
              ranges: HashMap::new(),
            });

          entry
            .ranges
            .insert(locale.to_string(), (interpolation.start, interpolation.end));

          if interpolation.typename != entry.typename {
            return Err(WoofError::InterpolationTypeMismatch);
          }
        }
      }

      Value::Table(table) => {
        let module = modules.entry(key.clone()).or_insert_with(|| Module {
          messages: BTreeMap::new(),
          modules: BTreeMap::new(),
        });

        build_module(locale, table, &mut module.messages, &mut module.modules)?;
      }

      _ => {
        return Err(WoofError::UnsupportedValueType {
          path: key.clone(),
          typename: value.type_str().to_string(),
        });
      }
    }
  }

  Ok(())
}

struct ParsedInterpolation {
  typename: Option<String>,
  name: String,
  start: usize,
  end: usize,
}

fn parse_interpolations(s: &str) -> Result<Vec<ParsedInterpolation>> {
  let mut result = Vec::new();
  let chars = s.chars().enumerate();

  let mut parsing_interpolation = false;
  let mut start = 0;
  let mut parsing_type = false;
  let mut current_name = String::new();
  let mut current_type = String::new();

  for (index, c) in chars {
    if c == '{' {
      start = index;
      parsing_interpolation = true;
      continue;
    }

    if !parsing_interpolation {
      continue;
    }

    // Skip escaped '{'
    if c == '{' && current_name.is_empty() {
      parsing_interpolation = false;
      continue;
    }

    if c == ':' {
      if current_name.is_empty() {
        return Err(WoofError::InvalidInterpolation(s.to_string()));
      }

      parsing_type = true;
      continue;
    }

    if c == '}' {
      if current_name.is_empty() {
        return Err(WoofError::InvalidInterpolation(s.to_string()));
      }

      let typename = if !current_type.is_empty() {
        let typename = current_type.clone();
        current_type.clear();
        Some(typename)
      } else {
        None
      };

      result.push(ParsedInterpolation {
        name: current_name.clone(),
        start,
        end: index,
        typename,
      });

      parsing_interpolation = false;
      parsing_type = false;
      current_name.clear();
      continue;
    }

    if parsing_type {
      current_type.push(c);
      continue;
    }

    current_name.push(c);
  }

  if parsing_interpolation {
    return Err(WoofError::InvalidInterpolation(s.to_string()));
  }

  Ok(result)
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
  /// Output directory for generated files
  #[arg(short, long, default_value = "messages")]
  out: String,

  /// Input directory containing translation files
  input_dir: String,
}

fn main() -> Result<()> {
  let config = Args::parse();
  let locales = collect_locales(config.input_dir)?;
  let modules = build_modules(locales)?;

  println!("{:#?}", modules);

  let out = Path::new(&config.out);
  if out.is_file() {
    return Err(WoofError::OutputFileExists(out.to_path_buf()));
  }

  // if out.exists() {
  //   fs::remove_dir_all(out)?;
  // }
  //
  // fs::create_dir_all(out)?;

  Ok(())
}
