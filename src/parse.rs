use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::Path;
use thiserror::Error;
use toml::{Table, Value};

pub type Result<T> = std::result::Result<T, WoofError>;

#[derive(Error, Debug)]
pub enum WoofError {
  #[error("IO error: {0}")]
  Io(#[from] std::io::Error),

  #[error("Path `{0}` is not a directory")]
  InvalidInputDirectory(String),

  #[error("Output file already exists at `{0}`")]
  OutputFileExists(std::path::PathBuf),

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

  #[error(transparent)]
  InterpolationError(#[from] ParseInterpolationError),
}

#[derive(Debug, Error)]
pub enum ParseInterpolationError {
  #[error("Unknown interpolation type `{0}`")]
  UnknownType(String),
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum InterpolationType {
  #[default]
  None,
  String,
  Number,
}

impl TryFrom<&str> for InterpolationType {
  type Error = ParseInterpolationError;

  fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
    match value {
      "string" => Ok(Self::String),
      "number" => Ok(Self::Number),
      _ => Err(ParseInterpolationError::UnknownType(value.to_string())),
    }
  }
}

impl std::fmt::Display for InterpolationType {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::None => write!(f, ""),
      Self::String => write!(f, "string"),
      Self::Number => write!(f, "number"),
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Locale(pub String);

impl std::hash::Hash for Locale {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.0.hash(state);
  }
}

impl std::fmt::Display for Locale {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "\"{}\"", self.0)
  }
}

#[derive(Debug, Clone)]
pub struct Key {
  pub literal: String,
  pub sanitized: String,
}

impl std::hash::Hash for Key {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.literal.hash(state);
  }
}

impl std::cmp::PartialEq for Key {
  fn eq(&self, other: &Self) -> bool {
    self.literal == other.literal
  }
}

impl std::cmp::Eq for Key {}

impl std::cmp::PartialOrd for Key {
  fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
    Some(self.literal.cmp(&other.literal))
  }
}

impl std::cmp::Ord for Key {
  fn cmp(&self, other: &Self) -> std::cmp::Ordering {
    self.literal.cmp(&other.literal)
  }
}

impl Key {
  pub fn new(literal: &str) -> Self {
    let sanitized = literal.to_string();
    Self {
      literal: literal.to_string(),
      sanitized,
    }
  }
}

#[derive(Debug, Clone)]
pub struct Translation {
  #[allow(dead_code)]
  pub literal: String,
  pub escaped: String,
}

impl Translation {
  pub fn new(literal: &str) -> Self {
    Self {
      literal: literal.to_string(),
      escaped: literal.to_string(),
    }
  }
}

#[derive(Debug, Default)]
pub struct Interpolation {
  pub type_: InterpolationType,
  pub ranges: HashMap<Locale, (usize, usize)>,
}

#[derive(Debug, Default)]
pub struct Message {
  pub translation: HashMap<Locale, Translation>,
  pub interpolations: HashMap<Key, Interpolation>,
}

#[derive(Debug)]
pub struct Module {
  pub messages: BTreeMap<Key, Message>,
  pub modules: BTreeMap<Key, Module>,
}

pub struct ParsedInterpolation {
  pub type_: InterpolationType,
  pub name: String,
  pub start: usize,
  pub end: usize,
}

pub fn collect_locales<P: AsRef<Path>>(dir: P) -> Result<HashMap<Locale, Value>> {
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
    let locale = Locale(stem.to_string());
    result.insert(locale, toml::from_str(&contents)?);
  }

  Ok(result)
}

pub fn build_modules(locales: HashMap<Locale, Value>) -> Result<Module> {
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
  locale: &Locale,
  table: Table,
  messages: &mut BTreeMap<Key, Message>,
  modules: &mut BTreeMap<Key, Module>,
) -> Result<()> {
  for (key, value) in table {
    let key = Key::new(&key);

    match value {
      Value::String(s) => {
        let interpolations = parse_interpolations(&s)?;
        let message = messages.entry(key.clone()).or_default();

        message
          .translation
          .insert(locale.clone(), Translation::new(&s));

        for interpolation in interpolations {
          let entry = message
            .interpolations
            .entry(Key::new(&interpolation.name))
            .or_default();

          entry
            .ranges
            .insert(locale.clone(), (interpolation.start, interpolation.end));

          if interpolation.type_ != entry.type_ {
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
          path: key.literal.clone(),
          typename: value.type_str().to_string(),
        });
      }
    }
  }

  Ok(())
}

pub fn parse_interpolations(s: &str) -> Result<Vec<ParsedInterpolation>> {
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
        InterpolationType::try_from(typename.as_str())?
      } else {
        InterpolationType::None
      };

      result.push(ParsedInterpolation {
        name: current_name.clone(),
        start,
        end: index,
        type_: typename,
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