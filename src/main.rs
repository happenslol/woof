mod generate;
mod parse;
mod sanitize;

use clap::Parser;
use std::path::Path;

use parse::{Result, build_modules, collect_locales};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
  /// Output directory for generated files
  #[arg(short, long, default_value = "messages")]
  out: String,

  /// Input directory containing translation files
  input_dir: String,
}

fn main() -> Result<()> {
  let config = Args::parse();
  let locales = collect_locales(config.input_dir)?;
  let locale_names = locales.keys().cloned().collect::<Vec<_>>();
  let modules = build_modules(locales)?;

  let out = Path::new(&config.out);
  generate::generate(out, locale_names.as_slice(), &modules)?;

  Ok(())
}
