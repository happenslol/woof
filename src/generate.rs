use crate::parse::{Locale, Module, Result, WoofError};
use std::fs;
use std::io::Write;
use std::iter::repeat_n;
use std::path::Path;

static DEFAULT_LOCALE: &str = "en";

pub fn generate(dir: &Path, locales: &[Locale], module: &Module) -> Result<()> {
  if dir.is_file() {
    return Err(WoofError::OutputFileExists(dir.to_path_buf()));
  }

  if dir.exists() {
    fs::remove_dir_all(dir)?;
  }

  fs::create_dir_all(dir)?;
  let locales_union = locales
    .iter()
    .map(|s| format!("\"{s}\""))
    .collect::<Vec<_>>()
    .join(" | ");

  fs::write(
    dir.join("index.ts"),
    format!(
      r#"let _locale = "{DEFAULT_LOCALE}"
export const setLocale = (locale: {locales_union}) => (_locale = locale)
export const getLocale = () => _locale
export * as m from "./root""#
    ),
  )?;

  write_module(dir, 0, module, &locales_union)
}

fn write_module(dir: &Path, depth: usize, module: &Module, locales: &str) -> Result<()> {
  let filename = if depth == 0 { "root.ts" } else { "index.ts" };
  let mut f = fs::File::create(dir.join(filename))?;

  let root_import = if depth == 0 {
    ".".to_string()
  } else {
    repeat_n("..", depth).collect::<Vec<&str>>().join("/")
  };

  writeln!(&mut f, "// eslint-disable")?;
  writeln!(&mut f, "import {{ getLocale }} from \"{root_import}\"")?;

  for (key, message) in module.messages.iter() {
    writeln!(
      &mut f,
      "export const {key} = (locale?: {locales}) => {{",
      key = key.sanitized
    )?;

    writeln!(&mut f, "  const resolved = locale ?? getLocale()")?;

    for (locale, string) in message.translation.iter() {
      writeln!(
        &mut f,
        "  if (resolved === \"{locale}\") return \"{}\"",
        string.escaped,
      )?;
    }

    writeln!(&mut f, "  return `{}`", key.sanitized)?;
    writeln!(&mut f, "}}")?;
  }

  for module_name in module.modules.keys() {
    writeln!(
      &mut f,
      "export * as {name} from \"./{name}\"",
      name = module_name.sanitized
    )?;
  }

  for (module_name, module) in module.modules.iter() {
    let dir = dir.join(&module_name.sanitized);
    fs::create_dir_all(&dir)?;
    write_module(&dir, depth + 1, module, locales)?;
  }

  Ok(())
}