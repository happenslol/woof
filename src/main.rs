mod collect;
mod context;
mod errors;
mod generate;
mod interpolations;
mod parse;
mod sanitize;

use clap::Parser;
use errors::WoofError;
use std::path::Path;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
  /// Output directory for generated files
  #[arg(short, long, default_value = "messages")]
  out: String,

  /// Input directory containing translation files
  input_dir: String,
}

fn main() -> Result<(), WoofError> {
  let config = Args::parse();
  let (modules, diagnostics) = collect::collect_and_build_modules(&config.input_dir)?;

  println!("{:#?}", diagnostics);

  // We need to collect locale names from the module structure
  let locale_names = collect_locale_names(&modules);

  let out = Path::new(&config.out);
  generate::generate(out, locale_names.as_slice(), &modules)?;

  Ok(())
}

// TODO: Do this while building modules
/// Recursively collects all locale names from a module structure
fn collect_locale_names(module: &parse::Module) -> Vec<parse::Locale> {
  let mut locales = std::collections::HashSet::new();
  collect_locale_names_recursive(module, &mut locales);
  locales.into_iter().collect()
}

fn collect_locale_names_recursive(
  module: &parse::Module,
  locales: &mut std::collections::HashSet<parse::Locale>,
) {
  // Collect locales from messages
  for message in module.messages.values() {
    for locale in message.translation.keys() {
      locales.insert(locale.clone());
    }
  }

  // Recursively collect from submodules
  for submodule in module.modules.values() {
    collect_locale_names_recursive(submodule, locales);
  }
}
