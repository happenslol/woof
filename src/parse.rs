use crate::{
  context::{Context, Diagnostics},
  interpolations::{Interpolation, parse_interpolations},
};
use std::collections::{BTreeMap, HashMap};
use toml::{Table, Value};

use crate::{
  errors::WoofError,
  sanitize::{escape_translation, sanitize_key},
};

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
pub struct Translation(pub String);

impl Translation {
  pub fn new(literal: &str) -> Self {
    let escaped = escape_translation(literal);
    Self(escaped)
  }
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

    // Replace escaped braces {{ with literal braces {
    // This is safe to do after interpolation replacement since all real
    // interpolations are now in ${args.name} format
    result = result.replace("{{", "{");

    Some(result)
  }
}

#[derive(Debug, Default)]
pub struct Module {
  pub messages: BTreeMap<Key, Message>,
  pub modules: BTreeMap<Key, Module>,
}

/// Builds a module from namespaced files by creating a parent module with namespace modules as
/// children
pub fn build_namespaced_module(
  namespaces: HashMap<String, HashMap<Locale, Value>>,
) -> Result<Module, WoofError> {
  let mut modules = std::collections::BTreeMap::new();

  for (namespace, locales) in namespaces {
    let module = build_flat_module(locales)?;
    let key = crate::parse::Key::new(&namespace);
    modules.insert(key, module);
  }

  Ok(Module {
    messages: std::collections::BTreeMap::new(),
    modules,
  })
}

pub fn build_flat_module(locales: HashMap<Locale, Value>) -> Result<Module, WoofError> {
  let mut root_module = Module::default();
  let mut diagnostics = Diagnostics::default();

  for (locale, value) in locales {
    let Value::Table(table) = value else {
      unreachable!("root is always a table");
    };

    let mut ctx = Context {
      locale: &locale,
      path: vec![],
      messages: &mut root_module.messages,
      modules: &mut root_module.modules,
      diagnostics: &mut diagnostics,
    };
    build_module(&mut ctx, table)?;
  }

  Ok(root_module)
}

fn build_module(ctx: &mut Context, table: Table) -> Result<(), WoofError> {
  for (key, value) in table {
    match value {
      Value::String(s) => {
        let translation = Translation::new(&s);
        let interpolations = parse_interpolations(&translation);
        if !interpolations.errors.is_empty() {
          ctx.add_interpolation_parse_errors(&key, interpolations.errors);
        }

        let message = ctx.messages.entry(Key::new(&key)).or_default();
        message
          .translation
          .insert(ctx.locale.clone(), Translation::new(&s));

        // We have to collect mismatches instead of adding them immediately because we still hold
        // a reference to the message
        // TODO: Smallvec?
        let mut mismatches = vec![];

        for interpolation in interpolations.interpolations {
          let entry = message
            .interpolations
            .entry(Key::new(&interpolation.name))
            .or_insert_with(|| Interpolation {
              type_: interpolation.type_,
              ranges: HashMap::with_capacity(1),
            });

          if interpolation.type_ != entry.type_ {
            mismatches.push((interpolation.type_, entry.clone()));
            continue;
          }

          entry
            .ranges
            .insert(ctx.locale.clone(), (interpolation.start, interpolation.end));
        }

        if !mismatches.is_empty() {
          ctx.add_interpolation_type_mismatches(&key, mismatches);
        }
      }

      Value::Table(table) => {
        let module = ctx.modules.entry(Key::new(&key)).or_default();
        let mut path = ctx.path.clone();
        path.push(&key);

        let mut ctx = Context {
          locale: ctx.locale,
          path,
          messages: &mut module.messages,
          modules: &mut module.modules,
          diagnostics: ctx.diagnostics,
        };

        build_module(&mut ctx, table)?;
      }

      _ => {
        ctx.add_unsupported_value_type(&key, value.type_str());
        continue;
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
    insta::assert_snapshot!(result.unwrap());
  }

  #[test]
  fn multibyte_characters_with_interpolation() {
    let test_interpolation = |input: &str| {
      let translation = Translation::new(input);
      let mut message = Message::default();
      let locale = Locale("en".to_string());

      let interpolations = parse_interpolations(&translation);
      message.translation.insert(locale.clone(), translation);

      // Add all found interpolations
      for interp in interpolations.interpolations {
        let mut interpolation_obj = Interpolation {
          type_: interp.type_,
          ..Default::default()
        };
        interpolation_obj
          .ranges
          .insert(locale.clone(), (interp.start, interp.end));
        message
          .interpolations
          .insert(Key::new(&interp.name), interpolation_obj);
      }

      message.template_for_locale(&locale).unwrap()
    };

    insta::assert_debug_snapshot!([
      test_interpolation("Hello 🌍 world! Welcome {name}!"),
      test_interpolation("Café {name}"),
      test_interpolation("中文 {count:number} 测试"),
      test_interpolation("🚀🌟✨ {msg} 🎉"),
      test_interpolation("Ñiño {age:number} años"),
      test_interpolation("👨‍👩‍👧‍👦 family {size:number}"),
    ]);
  }

  #[test]
  fn template_for_locale_sanitized_keys() {
    let mut message = Message::default();
    let locale = Locale("en".to_string());

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
    insta::assert_snapshot!(result.unwrap());
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
    insta::assert_snapshot!(result.unwrap());
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

    message
      .translation
      .insert(locale.clone(), Translation::new("Use `${var}` or {name}"));

    // The escaped version would be "Use \`\${var}\` or {name}"
    // So the interpolation position needs to account for the escaped string
    let mut name_interp = Interpolation::default();
    name_interp.ranges.insert(locale.clone(), (19, 24)); // {name} in escaped string
    message.interpolations.insert(Key::new("name"), name_interp);

    let result = message.template_for_locale(&locale);
    insta::assert_snapshot!(result.unwrap());
  }

  #[test]
  fn template_generation_edge_cases() {
    let generate = |input: &str| {
      let mut message = Message::default();
      let locale = Locale("en".to_string());

      let translation = Translation::new(input);
      let interpolations = parse_interpolations(&translation);
      assert!(interpolations.errors.is_empty());

      message.translation.insert(locale.clone(), translation);

      // Add all found interpolations
      for interp in interpolations.interpolations {
        let mut interpolation_obj = Interpolation {
          type_: interp.type_,
          ..Default::default()
        };
        interpolation_obj
          .ranges
          .insert(locale.clone(), (interp.start, interp.end));
        message
          .interpolations
          .insert(Key::new(&interp.name), interpolation_obj);
      }

      message.template_for_locale(&locale)
    };

    insta::assert_debug_snapshot!([
      generate(""),
      generate("No interpolations here"),
      generate("{single}"),
      generate("Only text no braces"),
      generate("Start {a} middle {b} end"),
      generate("{a}{b}{c}"),
      generate("Unicode 🌍 {name} more unicode 🎉"),
    ]);
  }

  #[test]
  fn brace_escapes_in_template_generation() {
    let generate = |input: &str| {
      let mut message = Message::default();
      let locale = Locale("en".to_string());

      let translation = Translation::new(input);
      let interpolations = parse_interpolations(&translation);
      assert!(interpolations.errors.is_empty());

      message.translation.insert(locale.clone(), translation);

      // Add all found interpolations
      for interp in interpolations.interpolations {
        let mut interpolation_obj = Interpolation {
          type_: interp.type_,
          ..Default::default()
        };
        interpolation_obj
          .ranges
          .insert(locale.clone(), (interp.start, interp.end));
        message
          .interpolations
          .insert(Key::new(&interp.name), interpolation_obj);
      }

      message.template_for_locale(&locale)
    };

    insta::assert_debug_snapshot!([
      generate("Welcome {{user} and {name}"),
      generate("Price: ${{amount} for {item}"),
      generate("Braces: {{} and {count:number}"),
      generate("Start {{literal} middle {var} end {{more}"),
      generate("Escape only {{starting double braces}}"),
    ]);
  }
}
