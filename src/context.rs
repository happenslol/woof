use std::collections::{BTreeMap, HashMap, HashSet};

use miette::Diagnostic;
use thiserror::Error;

use crate::{
  interpolations::{Interpolation, InterpolationParseError, InterpolationType},
  parse::{Key, Locale, Message, Module},
};

pub struct Context<'a> {
  pub locale: &'a Locale,
  pub path: Vec<&'a str>,
  pub messages: &'a mut BTreeMap<Key, Message>,
  pub modules: &'a mut BTreeMap<Key, Module>,
  pub diagnostics: &'a mut Diagnostics,
}

impl Context<'_> {
  pub fn add_unsupported_value_type(&mut self, key: &str, value_type: &str) {
    self
      .diagnostics
      .unsupported_value_types
      .push(UnsupportedValueType {
        locale: self.locale.clone(),
        path: self.path_at(key),
        value_type: value_type.to_string(),
      });
  }

  pub fn add_interpolation_type_mismatches(
    &mut self,
    key: &str,
    mismatches: Vec<(InterpolationType, Interpolation)>,
  ) {
    let key = self.path_at(key);
    let locale = self.locale.clone();

    for (found, existing) in mismatches.into_iter() {
      let entry = self
        .diagnostics
        .interpolation_type_mismatches
        .entry(key.clone())
        .or_default();

      // Insert existing locales
      for locale in existing.ranges.keys() {
        entry.insert((locale.clone(), found));
      }

      // Insert found type
      entry.insert((locale.clone(), found));
    }
  }

  pub fn add_interpolation_parse_errors(
    &mut self,
    key: &str,
    translation: &str,
    errors: Vec<InterpolationParseError>,
  ) {
    self
      .diagnostics
      .interpolation_errors
      .push(InterpolationError {
        locale: self.locale.clone(),
        translation: translation.to_string(),
        path: self.path_at(key),
        errors,
      });
  }

  fn path_at(&self, key: &str) -> String {
    self
      .path
      .iter()
      .chain(&[key])
      .cloned()
      .collect::<Vec<_>>()
      .join(".")
  }
}

#[derive(Debug, Error, Diagnostic)]
#[error("Unsupported value type at {locale}:{path}: {value_type}")]
pub struct UnsupportedValueType {
  locale: Locale,
  path: String,
  value_type: String,
}

#[derive(Debug, Error, Diagnostic)]
#[error("Interpolation at {locale}:{path} contains errors")]
#[diagnostic()]
pub struct InterpolationError {
  locale: Locale,
  path: String,

  #[source_code]
  translation: String,

  #[related]
  errors: Vec<InterpolationParseError>,
}

#[derive(Debug, Default, Error, Diagnostic)]
#[error("Errors found in translation files")]
#[diagnostic(code(some_code))]
pub struct Diagnostics {
  pub unsupported_value_types: Vec<UnsupportedValueType>,
  pub interpolation_type_mismatches: HashMap<String, HashSet<(Locale, InterpolationType)>>,
  pub interpolation_errors: Vec<InterpolationError>,
}

impl Diagnostics {
  pub fn is_empty(&self) -> bool {
    self.unsupported_value_types.is_empty()
      && self.interpolation_type_mismatches.is_empty()
      && self.interpolation_errors.is_empty()
  }
}
