use std::collections::HashMap;

use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

use crate::{
  parse::{Locale, Translation},
  sanitize::is_valid_identifier,
};

#[derive(Debug, Default, Clone)]
pub struct Interpolation {
  pub type_: InterpolationType,
  pub ranges: HashMap<Locale, (usize, usize)>,
}

#[derive(Debug)]
pub struct ParsedInterpolation {
  pub type_: InterpolationType,
  pub name: String,
  pub start: usize,
  pub end: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum InterpolationType {
  #[default]
  None,
  String,
  Number,
}

impl TryFrom<&str> for InterpolationType {
  type Error = ();

  fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
    match value {
      "string" => Ok(Self::String),
      "number" => Ok(Self::Number),
      _ => Err(()),
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

#[allow(dead_code)]
#[derive(Debug, Clone, Error, Diagnostic)]
pub enum InterpolationParseError {
  #[error("Empty interpolation identifier")]
  #[diagnostic(
    code(interpolation::empty),
    help = "Choose a valid name for this interpolation"
  )]
  Empty(#[label("There should be a name here")] SourceSpan),

  #[error("Invalid interpolation identifier")]
  #[diagnostic(
    code(interpolation::invalid_ident),
    help = "Identifiers must be valid javascript variables names"
  )]
  InvalidIdentifier(#[label("Contains invalid characters")] SourceSpan),

  #[error("Unclosed interpolation")]
  #[diagnostic(
    code(interpolation::unclosed),
    help = "Make sure your interpolations are properly closed"
  )]
  Unclosed(#[label("No closing brace")] SourceSpan),

  #[error("Invalid interpolation type")]
  #[diagnostic(
    code(interpolation::unsupported_type),
    help = "Choose a valid interpolation type"
  )]
  InvalidType {
    #[label("This type is not supported")]
    at: SourceSpan,
    type_: String,
  },
}

// TODO: Smallvecs?
#[derive(Debug, Default)]
pub struct ParsedInterpolations {
  pub interpolations: Vec<ParsedInterpolation>,
  pub errors: Vec<InterpolationParseError>,
}

pub fn parse_interpolations(translation: &Translation) -> ParsedInterpolations {
  let mut result = ParsedInterpolations::default();
  let s = &translation.0;

  if !s.contains('{') {
    return result;
  }

  let mut parsing_interpolation = false;
  let mut start_byte_index = 0;
  let mut parsing_type = false;
  let mut current_name = String::new();
  let mut current_type = String::new();

  let mut chars = s.char_indices().peekable();

  while let Some((byte_index, c)) = chars.next() {
    if c == '{' {
      // Check if this is an escape sequence {{
      if chars.peek().is_some_and(|&(_, next_char)| next_char == '{') {
        // Skip the escape sequence
        chars.next();
        continue;
      }

      if parsing_interpolation {
        // We're already parsing an interpolation and found another opening brace
        // This indicates nested braces, which is invalid

        // Skip until we hit the next closing brace, so we can keep parsing
        let mut offset = 0;
        while chars.peek().is_some_and(|&(_, c)| c != '}') {
          offset += 1;
          chars.next();
        }

        parsing_interpolation = false;
        parsing_type = false;
        current_name.clear();

        result
          .errors
          .push(InterpolationParseError::InvalidIdentifier(
            (start_byte_index + 1..byte_index + offset).into(),
          ));
        continue;
      }

      start_byte_index = byte_index;
      parsing_interpolation = true;
      continue;
    }

    if !parsing_interpolation {
      continue;
    }

    if c == ':' {
      if let Err(err) = validate_interpolation_name(start_byte_index, &current_name) {
        // Skip until we hit the next closing brace, so we can keep parsing
        while chars.peek().is_some_and(|&(_, c)| c != '}') {
          chars.next();
        }

        result.errors.push(err);
        parsing_interpolation = false;
        current_name.clear();
        continue;
      };

      parsing_type = true;
      continue;
    }

    if c == '}' {
      // This is the end of the interpolation
      let typename = if !current_type.is_empty() {
        let type_ = match InterpolationType::try_from(current_type.as_str()) {
          Ok(t) => t,
          Err(()) => {
            result.errors.push(InterpolationParseError::InvalidType {
              at: (start_byte_index + current_name.len() + 2..byte_index).into(),
              type_: current_type.clone(),
            });

            parsing_interpolation = false;
            parsing_type = false;
            current_name.clear();
            current_type.clear();
            continue;
          }
        };

        current_type.clear();
        type_
      } else {
        // Only validate if we haven't already done so (when no type was specified)
        match validate_interpolation_name(start_byte_index, &current_name) {
          Ok(_) => InterpolationType::None,
          Err(err) => {
            parsing_interpolation = false;
            parsing_type = false;
            current_name.clear();
            result.errors.push(err);
            continue;
          }
        }
      };

      result.interpolations.push(ParsedInterpolation {
        name: current_name.clone(),
        start: start_byte_index,
        end: byte_index,
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
    // Unclosed interpolation
    result.errors.push(InterpolationParseError::Unclosed(
      (start_byte_index + 1..s.len()).into(),
    ));
  }

  result
}

/// Validates that an interpolation identifier follows the rules:
/// - Must start with a letter (a-z, A-Z)
/// - Can only contain alphanumeric characters and underscores
fn validate_interpolation_name(start: usize, name: &str) -> Result<(), InterpolationParseError> {
  if name.is_empty() {
    return Err(InterpolationParseError::Empty(start.into()));
  }

  if !is_valid_identifier(name) {
    return Err(InterpolationParseError::InvalidIdentifier(
      (start + 1, name.len()).into(),
    ));
  }

  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  fn parse(input: &str) -> ParsedInterpolations {
    let translation = Translation::new(input);
    parse_interpolations(&translation)
  }

  #[test]
  fn valid_interpolation_identifiers() {
    insta::assert_debug_snapshot!([
      parse("Hello {name}"),
      parse("Count: {count:number}"),
      parse("User {userId}"),
      parse("Value {value_123}"),
      parse("Test {a}"),
      parse("Multiple {firstName} {lastName}"),
      parse("Underscore {user_name}"),
      parse("Mixed {value1} and {item_2}"),
    ]);
  }

  #[test]
  fn invalid_interpolation_identifiers() {
    insta::assert_debug_snapshot!([
      parse("Number start {123name}"),
      parse("Hyphen {user-name}"),
      parse("Space {user name}"),
      parse("Dot {user.name}"),
      parse("Special chars {user@email}"),
      parse("Underscore start {_name}"),
      parse("Number only {123}"),
      parse("Special start {$var}"),
      parse("Unicode {名前}"),
    ]);
  }

  #[test]
  fn interpolation_edge_cases() {
    insta::assert_debug_snapshot!([
      parse("{}"),
      parse("{:string}"),
      parse("{a}{b}{c}"),
      parse("{a}and{b}"),
      parse("{outer{inner}}"),
      parse("\\{invalid_interpolation\\}"),
      parse("{{not_interpolation}}"),
      parse("} and { separate"),
      parse("{name without closing"),
    ]);
  }

  #[test]
  fn complex_interpolation_scenarios() {
    insta::assert_debug_snapshot!([
      parse("Mixed types: {name:string} has {count:number} items"),
      parse("Long interpolation names: {veryLongInterpolationNameThatShouldStillWork:string}"),
      parse("Multiple same type: {first:string} and {second:string} and {third:string}"),
      parse("Interpolations with unicode text: 🎉 {celebration:string} 🎊 {party:number} 🥳"),
      parse("Interpolations at boundaries: {start}middle text{end}"),
      parse("Only interpolations: {a}{b}{c}{d}"),
    ]);
  }

  #[test]
  fn brace_escape_sequences() {
    insta::assert_debug_snapshot!([
      parse("{{hello}}"),
      parse("{name} and {{literal}}"),
      parse("{{}} here"),
      parse("{{start}} {name}"),
      parse("{{first}} {{second}}"),
      parse("{{text}} with {name:string} and {{more}}"),
      parse("Just } here"),
      parse("{{start"),
      parse("{name} test"),
      parse("{{{{"),
    ]);
  }
}
