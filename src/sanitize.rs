use std::collections::HashSet;

/// Sanitizes a key to ensure it's a valid JavaScript/TypeScript identifier.
///
/// Rules:
/// - Only alphanumeric characters (a-z, A-Z, 0-9) and underscores are allowed
/// - Reserved JavaScript/TypeScript keywords get an underscore appended
/// - Keys starting with numbers get an underscore prepended
/// - All other characters are removed
pub fn sanitize_key(key: &str) -> String {
  // First, remove any non-alphanumeric characters (except underscores)
  let mut sanitized: String = key
    .chars()
    .filter(|c| c.is_alphanumeric() || *c == '_')
    .collect();

  // If the key starts with a number, prepend an underscore
  if sanitized.chars().next().is_some_and(|c| c.is_numeric()) {
    sanitized = format!("_{}", sanitized);
  }

  // Check if it's a reserved keyword and append underscore if needed
  if is_reserved_keyword(&sanitized) {
    sanitized.push('_');
  }

  sanitized
}

/// Checks if a string is a reserved JavaScript/TypeScript keyword
fn is_reserved_keyword(word: &str) -> bool {
  static KEYWORDS: &[&str] = &[
    // JavaScript reserved keywords
    "abstract",
    "arguments",
    "await",
    "boolean",
    "break",
    "byte",
    "case",
    "catch",
    "char",
    "class",
    "const",
    "continue",
    "debugger",
    "default",
    "delete",
    "do",
    "double",
    "else",
    "enum",
    "eval",
    "export",
    "extends",
    "false",
    "final",
    "finally",
    "float",
    "for",
    "function",
    "goto",
    "if",
    "implements",
    "import",
    "in",
    "instanceof",
    "int",
    "interface",
    "let",
    "long",
    "native",
    "new",
    "null",
    "package",
    "private",
    "protected",
    "public",
    "return",
    "short",
    "static",
    "super",
    "switch",
    "synchronized",
    "this",
    "throw",
    "throws",
    "transient",
    "true",
    "try",
    "typeof",
    "var",
    "void",
    "volatile",
    "while",
    "with",
    "yield",
    // TypeScript additional keywords
    "any",
    "as",
    "async",
    "asserts",
    "bigint",
    "constructor",
    "declare",
    "from",
    "get",
    "infer",
    "is",
    "keyof",
    "module",
    "namespace",
    "never",
    "readonly",
    "require",
    "set",
    "string",
    "symbol",
    "type",
    "undefined",
    "unique",
    "unknown",
    "using",
    // Common global objects that should be avoided
    "Array",
    "Boolean",
    "Date",
    "Error",
    "Function",
    "JSON",
    "Math",
    "Number",
    "Object",
    "Promise",
    "RegExp",
    "String",
    "Symbol",
    "Map",
    "Set",
    "WeakMap",
    "WeakSet",
    // Common browser/Node.js globals
    "console",
    "window",
    "document",
    "process",
    "global",
    "Buffer",
    "module",
    "exports",
    "require",
    "__dirname",
    "__filename",
  ];

  static KEYWORDS_SET: std::sync::OnceLock<HashSet<&'static str>> = std::sync::OnceLock::new();

  let keywords = KEYWORDS_SET.get_or_init(|| KEYWORDS.iter().copied().collect());
  keywords.contains(word)
}

/// Escapes a translation string for use in JavaScript template literals.
///
/// This function escapes special characters that have meaning in template literals
/// while preserving interpolation patterns like `{name}` or `{name:type}`.
///
/// Characters that are escaped:
/// - Backticks (`) become \`
/// - Backslashes (\) become \\
/// - Dollar signs followed by curly braces (${) become \${
pub fn escape_translation(s: &str) -> String {
  let mut result = String::with_capacity(s.len());
  let chars: Vec<char> = s.chars().collect();
  let mut i = 0;

  while i < chars.len() {
    match chars[i] {
      '`' => result.push_str("\\`"),
      '\\' => result.push_str("\\\\"),
      '$' => {
        // Check if this is followed by {
        if i + 1 < chars.len() && chars[i + 1] == '{' {
          result.push_str("\\$");
        } else {
          result.push('$');
        }
      }
      _ => result.push(chars[i]),
    }
    i += 1;
  }

  result
}

/// Checks whether a string matches `[a-zA-Z_][a-zA-Z0-9_]*`
pub fn is_valid_identifier(s: &str) -> bool {
  let mut chars = s.chars();

  // First character must be a letter
  if !chars.next().is_some_and(|c| c.is_ascii_alphabetic()) {
    return false;
  }

  // Remaining characters must be alphanumeric or underscore
  chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn alphanumeric_keys() {
    insta::assert_debug_snapshot!([
      sanitize_key("hello"),
      sanitize_key("hello123"),
      sanitize_key("HELLO_WORLD"),
      sanitize_key("test_123_ABC"),
    ]);
  }

  #[test]
  fn non_alphanumeric_replacement() {
    insta::assert_debug_snapshot!([
      sanitize_key("hello-world"),
      sanitize_key("hello.world"),
      sanitize_key("hello world"),
      sanitize_key("hello@world!"),
      sanitize_key("test-key-123"),
      sanitize_key("hello---world"),
      sanitize_key("test...key"),
      sanitize_key("multiple___underscores"),
      sanitize_key("@@##$$%%"),
      sanitize_key("mix-.-_-.-test"),
    ]);
  }

  #[test]
  fn reserved_keywords() {
    insta::assert_debug_snapshot!([
      sanitize_key("class"),
      sanitize_key("function"),
      sanitize_key("return"),
      sanitize_key("const"),
      sanitize_key("let"),
      sanitize_key("async"),
      sanitize_key("await"),
      sanitize_key("Promise"),
      sanitize_key("Class"),
      sanitize_key("FUNCTION"),
      sanitize_key("ASYNC"),
    ]);
  }

  #[test]
  fn numeric_start() {
    insta::assert_debug_snapshot!([
      sanitize_key("123hello"),
      sanitize_key("1test"),
      sanitize_key("999"),
    ]);
  }

  #[test]
  fn edge_cases() {
    assert_eq!(sanitize_key(""), "");
    assert_eq!(sanitize_key("@@@"), "");
    assert_eq!(sanitize_key("   "), "");

    insta::assert_debug_snapshot!([sanitize_key("___"),]);
  }

  #[test]
  fn combined_rules() {
    insta::assert_debug_snapshot!([
      sanitize_key("123-class"),
      sanitize_key("my-function-name"),
      sanitize_key("async-operation"),
      sanitize_key("get-data"),
    ]);
  }

  #[test]
  fn escape_basic_strings() {
    insta::assert_debug_snapshot!([
      escape_translation("hello world"),
      escape_translation("simple string"),
    ]);

    assert_eq!(escape_translation(""), "");
  }

  #[test]
  fn escape_backticks() {
    insta::assert_debug_snapshot!([
      escape_translation("hello `world`"),
      escape_translation("`backtick at start"),
      escape_translation("backtick at end`"),
      escape_translation("```backticks```"),
    ]);
  }

  #[test]
  fn escape_backslashes() {
    insta::assert_debug_snapshot!([
      escape_translation("path\\to\\file"),
      escape_translation("escape \\n newline"),
      escape_translation("\\"),
      escape_translation("\\\\\\\\"),
    ]);
  }

  #[test]
  fn escape_dollar_brace() {
    insta::assert_debug_snapshot!([
      escape_translation("price: ${amount}"),
      escape_translation("${start} to ${end}"),
      escape_translation("just $ dollar"),
      escape_translation("$"),
      escape_translation("$notbrace"),
      escape_translation("$$$${multiple}"),
    ]);
  }

  #[test]
  fn preserve_interpolations() {
    insta::assert_debug_snapshot!([
      escape_translation("Hello {name}"),
      escape_translation("Count: {count:number}"),
      escape_translation("{greeting:string}, {name}!"),
      escape_translation("Multiple {a} and {b:number} interpolations"),
    ]);
  }

  #[test]
  fn combined_escaping() {
    insta::assert_debug_snapshot!([
      escape_translation("Use `${var}` or {name}"),
      escape_translation("Path: C:\\Users\\{username}"),
      escape_translation("`Hello ${world}` says {name:string}"),
      escape_translation("\\`${}\\`"),
      escape_translation("`\\${test}\\`"),
      escape_translation("Before{var}\\after`"),
      escape_translation("`${start}{middle:type}${end}`"),
    ]);
  }

  #[test]
  fn unicode_and_non_ascii_keys() {
    insta::assert_debug_snapshot!([
      sanitize_key("café"),
      sanitize_key("naïve"),
      sanitize_key("测试"),
      sanitize_key("🚀rocket"),
      sanitize_key("mix_中文_test"),
      sanitize_key("émoji🎉test"),
    ]);
  }

  #[test]
  fn brace_escape_sequences() {
    insta::assert_debug_snapshot!([
      escape_translation("Literal braces {{hello}}"),
      escape_translation("Mixed {name} and {{literal}}"),
      escape_translation("Only escapes {{}} here"),
      escape_translation("With template literal ${{var}}"),
      escape_translation("Complex {{start}} ${middle} {{end}}"),
      escape_translation("Brace escapes with backticks `{{test}}`"),
      escape_translation("Single } characters are fine"),
    ]);
  }
}
