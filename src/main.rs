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
  let result = collect::collect_and_build_modules(&config.input_dir)?;

  if !result.diagnostics.is_empty() {
    let handler = miette::GraphicalReportHandler::new().with_show_related_as_nested(true);
    let mut out = String::new();
    handler
      .render_report(&mut out, &result.diagnostics)
      .unwrap();
    println!("{}", out);
  }

  let out = Path::new(&config.out);
  generate::generate(out, &result.locales, &result.module)?;

  Ok(())
}
