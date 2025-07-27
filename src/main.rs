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
  let args = Args::parse();
  let result = collect::collect_and_build_modules(&args.input_dir)?;
  result.diagnostics.report();

  let out = Path::new(&args.out);
  generate::generate(out, &result.locales, &result.module)?;

  Ok(())
}
