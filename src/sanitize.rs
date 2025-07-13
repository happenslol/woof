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
  if sanitized.chars().next().map_or(false, |c| c.is_numeric()) {
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn alphanumeric_keys() {
    assert_eq!(sanitize_key("hello"), "hello");
    assert_eq!(sanitize_key("hello123"), "hello123");
    assert_eq!(sanitize_key("HELLO_WORLD"), "HELLO_WORLD");
    assert_eq!(sanitize_key("test_123_ABC"), "test_123_ABC");
  }

  #[test]
  fn non_alphanumeric_replacement() {
    assert_eq!(sanitize_key("hello-world"), "helloworld");
    assert_eq!(sanitize_key("hello.world"), "helloworld");
    assert_eq!(sanitize_key("hello world"), "helloworld");
    assert_eq!(sanitize_key("hello@world!"), "helloworld");
    assert_eq!(sanitize_key("test-key-123"), "testkey123");
  }

  #[test]
  fn reserved_keywords() {
    assert_eq!(sanitize_key("class"), "class_");
    assert_eq!(sanitize_key("function"), "function_");
    assert_eq!(sanitize_key("return"), "return_");
    assert_eq!(sanitize_key("const"), "const_");
    assert_eq!(sanitize_key("let"), "let_");
    assert_eq!(sanitize_key("async"), "async_");
    assert_eq!(sanitize_key("await"), "await_");
    assert_eq!(sanitize_key("Promise"), "Promise_");
  }

  #[test]
  fn numeric_start() {
    assert_eq!(sanitize_key("123hello"), "_123hello");
    assert_eq!(sanitize_key("1test"), "_1test");
    assert_eq!(sanitize_key("999"), "_999");
  }

  #[test]
  fn edge_cases() {
    assert_eq!(sanitize_key(""), "");
    assert_eq!(sanitize_key("___"), "___");
    assert_eq!(sanitize_key("@@@"), "");
    assert_eq!(sanitize_key("   "), "");
  }

  #[test]
  fn combined_rules() {
    assert_eq!(sanitize_key("123-class"), "_123class");
    assert_eq!(sanitize_key("my-function-name"), "myfunctionname");
    assert_eq!(sanitize_key("async-operation"), "asyncoperation");
    assert_eq!(sanitize_key("get-data"), "getdata");
  }

  #[test]
  fn escape_basic_strings() {
    assert_eq!(escape_translation("hello world"), "hello world");
    assert_eq!(escape_translation("simple string"), "simple string");
    assert_eq!(escape_translation(""), "");
  }

  #[test]
  fn escape_backticks() {
    assert_eq!(escape_translation("hello `world`"), "hello \\`world\\`");
    assert_eq!(
      escape_translation("`backtick at start"),
      "\\`backtick at start"
    );
    assert_eq!(escape_translation("backtick at end`"), "backtick at end\\`");
  }

  #[test]
  fn escape_backslashes() {
    assert_eq!(escape_translation("path\\to\\file"), "path\\\\to\\\\file");
    assert_eq!(
      escape_translation("escape \\n newline"),
      "escape \\\\n newline"
    );
    assert_eq!(escape_translation("\\"), "\\\\");
  }

  #[test]
  fn escape_dollar_brace() {
    assert_eq!(escape_translation("price: ${amount}"), "price: \\${amount}");
    assert_eq!(
      escape_translation("${start} to ${end}"),
      "\\${start} to \\${end}"
    );
    assert_eq!(escape_translation("just $ dollar"), "just $ dollar");
    assert_eq!(escape_translation("$"), "$");
    assert_eq!(escape_translation("$notbrace"), "$notbrace");
  }

  #[test]
  fn preserve_interpolations() {
    assert_eq!(escape_translation("Hello {name}"), "Hello {name}");
    assert_eq!(
      escape_translation("Count: {count:number}"),
      "Count: {count:number}"
    );
    assert_eq!(
      escape_translation("{greeting:string}, {name}!"),
      "{greeting:string}, {name}!"
    );
    assert_eq!(
      escape_translation("Multiple {a} and {b:number} interpolations"),
      "Multiple {a} and {b:number} interpolations"
    );
  }

  #[test]
  fn combined_escaping() {
    assert_eq!(
      escape_translation("Use `${var}` or {name}"),
      "Use \\`\\${var}\\` or {name}"
    );
    assert_eq!(
      escape_translation("Path: C:\\Users\\{username}"),
      "Path: C:\\\\Users\\\\{username}"
    );
    assert_eq!(
      escape_translation("`Hello ${world}` says {name:string}"),
      "\\`Hello \\${world}\\` says {name:string}"
    );
  }
}
