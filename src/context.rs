use std::collections::{BTreeMap, HashMap, HashSet};

use miette::Diagnostic;
use owo_colors::OwoColorize;
use thiserror::Error;

use crate::{
  interpolations::{Interpolation, InterpolationParseError, InterpolationType},
  parse::{Key, Locale, Message, Module},
};

pub struct Context<'a> {
  pub locale: &'a Locale,
  pub normalized_file_path: &'a str,
  pub key_path: Vec<&'a str>,
  pub messages: &'a mut BTreeMap<Key, Message>,
  pub modules: &'a mut BTreeMap<Key, Module>,
  pub diagnostics: &'a mut Diagnostics,
}

impl Context<'_> {
  pub fn add_interpolation_type_mismatches(
    &mut self,
    key: &str,
    mismatches: Vec<(String, InterpolationType, Interpolation)>,
  ) {
    let key = self.path_at(key);
    let locale = self.locale.clone();

    for (name, found, existing) in mismatches.into_iter() {
      let entry = self
        .diagnostics
        .interpolation_type_mismatches
        .entry((key.clone(), name.clone()))
        .or_default();

      // Insert existing locales
      for locale in existing.ranges.keys() {
        entry.insert((locale.clone(), existing.type_));
      }

      // Insert found type
      entry.insert((locale.clone(), found));
    }
  }

  pub fn add_key_diagnostics(&mut self, key: &str, diagnostic: KeyDiagnostic) {
    let key = self.path_at(key);
    let locale = self.locale.clone();
    let normalized_file_path = self.normalized_file_path.to_string();

    let file_diagnostics = self
      .diagnostics
      .file_diagnostics
      .entry((locale, normalized_file_path))
      .or_default();

    file_diagnostics.insert(key.clone(), diagnostic);
  }

  fn path_at(&self, key: &str) -> String {
    self
      .key_path
      .iter()
      .chain(&[key])
      .cloned()
      .collect::<Vec<_>>()
      .join(".")
  }
}

#[derive(Debug, Default)]
pub struct Diagnostics {
  pub file_diagnostics: HashMap<(Locale, String), HashMap<String, KeyDiagnostic>>,
  pub interpolation_type_mismatches:
    HashMap<(String, String), HashSet<(Locale, InterpolationType)>>,
}

#[derive(Debug, Clone, Error, Diagnostic)]
pub enum KeyDiagnostic {
  #[error("Unsupported value type: {}", value_type.purple())]
  #[diagnostic()]
  UnsupportedValueType { value_type: String },

  #[error("Interpolation errors found")]
  #[diagnostic()]
  InterpolationErrors {
    #[source_code]
    source_code: String,
    #[related]
    errors: Vec<InterpolationParseError>,
  },
}

impl Diagnostics {
  pub fn is_empty(&self) -> bool {
    self.file_diagnostics.is_empty() && self.interpolation_type_mismatches.is_empty()
  }

  pub fn report(&self) {
    if self.is_empty() {
      return;
    }

    let handler = miette::GraphicalReportHandler::new().with_show_related_as_nested(true);
    let mut buf = String::new();

    for ((_locale, file), diagnostics) in self.file_diagnostics.iter() {
      eprintln!("Errors in {}:", file.green());

      for (key, diagnostic) in diagnostics {
        buf.clear();
        println!("Errors in key {}:", key.yellow());
        let _ = handler.render_report(&mut buf, diagnostic);
        eprintln!("{buf}");
      }
    }

    for ((key, name), mismatches) in self.interpolation_type_mismatches.iter() {
      eprintln!(
        "Interpolation {} in key {} has different types between locales:",
        name.cyan(),
        key.yellow()
      );

      for (locale, type_) in mismatches {
        eprintln!(
          "  • Locale {} defines type as: {}",
          locale.blue(),
          type_.purple()
        );
      }

      eprintln!();
    }
  }
}
