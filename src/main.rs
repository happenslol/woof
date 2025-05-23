use std::{
  collections::BTreeMap,
  error::Error,
  fs::{File, create_dir_all},
  io::{Read, Write},
  sync::LazyLock,
};

use regex::Regex;

#[derive(Debug, Clone)]
enum Entry<'a> {
  String(&'a str),
  Object(BTreeMap<&'a str, Entry<'a>>),
}

fn main() -> Result<(), Box<dyn Error>> {
  let mut f = File::open("./example.locale")?;
  let mut str = String::new();
  f.read_to_string(&mut str)?;
  let mut values = BTreeMap::new();

  for line in str.lines() {
    if line.trim().is_empty() {
      continue;
    }

    if line.starts_with("#") {
      continue;
    }

    let mut parts = line.split("=");
    let Some(key) = parts.next() else {
      eprintln!("Invalid line: {line}");
      continue;
    };

    let Some(value) = parts.next() else {
      eprintln!("Invalid line: {line}");
      continue;
    };

    insert_entry(&mut values, key, value);
  }

  create_dir_all("./out")?;
  let mut out = File::create("./out/translations.ts")?;

  writeln!(out, "export const t = {{")?;
  write_map(&mut out, &values)?;
  writeln!(out, "}};")?;

  Ok(())
}

fn insert_entry<'a>(map: &mut BTreeMap<&'a str, Entry<'a>>, key: &'a str, value: &'a str) {
  let key = key.trim();
  let value = value.trim();

  let mut current_map = map;
  let mut parts = key.split('.');

  let mut current_key = parts.next().unwrap();
  for part in parts {
    current_map
      .entry(current_key)
      .or_insert(Entry::Object(BTreeMap::new()));
    current_map = match current_map.get_mut(current_key).unwrap() {
      Entry::Object(map) => map,
      _ => unreachable!(),
    };
    current_key = part;
  }

  current_map.insert(current_key, Entry::String(value));
}

fn write_map(out: &mut File, map: &BTreeMap<&str, Entry>) -> Result<(), Box<dyn Error>> {
  for (key, value) in map {
    match value {
      Entry::String(value) => write_key(out, key, value)?,
      Entry::Object(map) => {
        writeln!(out, "{key}: {{")?;
        write_map(out, map)?;
        writeln!(out, "}},")?;
      }
    }
  }

  Ok(())
}

static VALUE_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\{([^}]+)\}").unwrap());

fn write_key(out: &mut File, key: &str, value: &str) -> Result<(), Box<dyn Error>> {
  if value.contains('{') {
    writeln!(out, "{key}: (values: {{")?;

    let captures = VALUE_REGEX.captures_iter(value);
    for capture in captures {
      let Some(key) = capture.get(1) else {
        continue;
      };

      let key = key.as_str();
      write!(out, "{key}: string,")?;
    }

    let repl = VALUE_REGEX.replace_all(value, "$${values.$1}");
    writeln!(out, "}}) => `{repl}`,")?;
    return Ok(());
  }

  writeln!(out, "{key}: () => `{value}`,")?;
  Ok(())
}
