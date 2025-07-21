use std::collections::{BTreeMap, HashMap};
use thiserror::Error;
use toml::{Table, Value};

use crate::sanitize::{escape_translation, sanitize_key};

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

  #[error("Mixed file naming modes detected: found both flat and namespaced files")]
  MixedFileModes,

  #[error("Invalid namespaced file name: {0} (expected format: namespace.locale.toml)")]
  InvalidNamespacedFileName(String),

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
      Self::None => write!(f, "none"),
      Self::String => write!(f, "string"),
      Self::Number => write!(f, "number"),
    }
  }
}

impl InterpolationType {
  pub fn as_typescript_type(&self) -> &'static str {
    match self {
      Self::None => "string",
      Self::String => "string",
      Self::Number => "number",
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Locale(pub String);

impl std::hash::Hash for Locale {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.0.hash(state);
  }
}

impl std::fmt::Display for Locale {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.0)
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
    Self {
      literal: literal.to_string(),
      sanitized: sanitize_key(literal),
    }
  }
}

#[derive(Debug, Clone)]
pub struct Translation(String);

impl Translation {
  pub fn new(literal: &str) -> Self {
    let escaped = escape_translation(literal);
    Self(escaped)
  }

  pub fn parse_interpolations(&self) -> Result<Vec<ParsedInterpolation>> {
    let s = &self.0;
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
}

#[derive(Debug, Default)]
pub struct Interpolation {
  pub type_: InterpolationType,
  pub ranges: HashMap<Locale, (usize, usize)>,
}

#[derive(Debug, Default)]
pub struct Message {
  pub translation: BTreeMap<Locale, Translation>,
  pub interpolations: BTreeMap<Key, Interpolation>,
}

impl Message {
  /// Creates a template string for the given locale by replacing interpolations
  /// with JavaScript template literal syntax `${name}`.
  ///
  /// The interpolations are replaced in the escaped string, maintaining proper
  /// offsets as the string length changes during replacement.
  pub fn template_for_locale(&self, locale: &Locale) -> Option<String> {
    // Get the translation for this locale
    let translation = self.translation.get(locale)?;
    let mut result = translation.0.clone();

    // Collect all interpolations for this locale and sort by start position
    let mut interpolations: Vec<(&Key, (usize, usize))> = self
      .interpolations
      .iter()
      .filter_map(|(key, interp)| interp.ranges.get(locale).map(|&range| (key, range)))
      .collect();

    // Sort by start position (ascending)
    interpolations.sort_by_key(|(_, (start, _))| *start);

    // Replace interpolations from back to front to avoid offset issues
    // Reverse so we process from end to start
    interpolations.reverse();

    for (key, (start, end)) in interpolations {
      let template_var = format!("${{args.{}}}", key.sanitized);
      result.replace_range(start..=end, &template_var);
    }

    Some(result)
  }
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

/// Builds a module from namespaced files by creating a parent module with namespace modules as
/// children
pub fn build_namespaced_module(
  namespaced_groups: HashMap<String, HashMap<Locale, Value>>,
) -> Result<Module> {
  let mut modules = std::collections::BTreeMap::new();

  for (namespace, locales) in namespaced_groups {
    let module = build_flat_module(locales)?;
    let key = crate::parse::Key::new(&namespace);
    modules.insert(key, module);
  }

  Ok(Module {
    messages: std::collections::BTreeMap::new(),
    modules,
  })
}

pub fn build_flat_module(locales: HashMap<Locale, Value>) -> Result<Module> {
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
        let message = messages.entry(key.clone()).or_default();
        let translation = Translation::new(&s);
        let interpolations = translation.parse_interpolations()?;

        message
          .translation
          .insert(locale.clone(), Translation::new(&s));

        for interpolation in interpolations {
          let entry = message
            .interpolations
            .entry(Key::new(&interpolation.name))
            .or_insert_with(|| Interpolation {
              type_: interpolation.type_,
              ranges: HashMap::with_capacity(1),
            });

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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn template_for_locale_basic() {
    let mut message = Message::default();
    let locale = Locale("en".to_string());

    // Add a translation with interpolations
    message.translation.insert(
      locale.clone(),
      Translation::new("Hello {name}, you have {count} messages"),
    );

    // Add interpolation info
    let mut name_interp = Interpolation::default();
    name_interp.ranges.insert(locale.clone(), (6, 11)); // {name}
    message.interpolations.insert(Key::new("name"), name_interp);

    let mut count_interp = Interpolation::default();
    count_interp.ranges.insert(locale.clone(), (23, 29)); // {count}
    message
      .interpolations
      .insert(Key::new("count"), count_interp);

    let result = message.template_for_locale(&locale);
    assert_eq!(
      result,
      Some("Hello ${name}, you have ${count} messages".to_string())
    );
  }

  #[test]
  fn template_for_locale_sanitized_keys() {
    let mut message = Message::default();
    let locale = Locale("en".to_string());

    // Add a translation with interpolations that need sanitization
    message.translation.insert(
      locale.clone(),
      Translation::new("Class: {class}, function: {function}"),
    );

    // Add interpolation info for reserved keywords
    let mut class_interp = Interpolation::default();
    class_interp.ranges.insert(locale.clone(), (7, 13)); // {class}
    message
      .interpolations
      .insert(Key::new("class"), class_interp);

    let mut func_interp = Interpolation::default();
    func_interp.ranges.insert(locale.clone(), (26, 35)); // {function}
    message
      .interpolations
      .insert(Key::new("function"), func_interp);

    let result = message.template_for_locale(&locale);
    assert_eq!(
      result,
      Some("Class: ${class_}, function: ${function_}".to_string())
    );
  }

  #[test]
  fn template_for_locale_multiple_interpolations() {
    let mut message = Message::default();
    let locale = Locale("en".to_string());

    // Test with multiple interpolations to ensure correct ordering
    message
      .translation
      .insert(locale.clone(), Translation::new("{a} {b} {c} {d}"));

    // Add interpolations in non-sequential order to test sorting
    let mut d_interp = Interpolation::default();
    d_interp.ranges.insert(locale.clone(), (12, 14)); // {d}
    message.interpolations.insert(Key::new("d"), d_interp);

    let mut b_interp = Interpolation::default();
    b_interp.ranges.insert(locale.clone(), (4, 6)); // {b}
    message.interpolations.insert(Key::new("b"), b_interp);

    let mut a_interp = Interpolation::default();
    a_interp.ranges.insert(locale.clone(), (0, 2)); // {a}
    message.interpolations.insert(Key::new("a"), a_interp);

    let mut c_interp = Interpolation::default();
    c_interp.ranges.insert(locale.clone(), (8, 10)); // {c}
    message.interpolations.insert(Key::new("c"), c_interp);

    let result = message.template_for_locale(&locale);
    assert_eq!(result, Some("${a} ${b} ${c} ${d}".to_string()));
  }

  #[test]
  fn template_for_locale_missing_locale() {
    let message = Message::default();
    let locale = Locale("fr".to_string());

    let result = message.template_for_locale(&locale);
    assert_eq!(result, None);
  }

  #[test]
  fn template_for_locale_with_escaping() {
    let mut message = Message::default();
    let locale = Locale("en".to_string());

    // Add a translation that needs escaping
    message
      .translation
      .insert(locale.clone(), Translation::new("Use `${var}` or {name}"));

    // The escaped version would be "Use \`\${var}\` or {name}"
    // So the interpolation position needs to account for the escaped string
    let mut name_interp = Interpolation::default();
    name_interp.ranges.insert(locale.clone(), (19, 24)); // {name} in escaped string
    message.interpolations.insert(Key::new("name"), name_interp);

    let result = message.template_for_locale(&locale);
    assert_eq!(result, Some("Use \\`\\${var}\\` or ${name}".to_string()));
  }
}
