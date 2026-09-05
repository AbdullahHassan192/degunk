use clap::Parser;
use degunk::cli::Cli;
use degunk::ui::{run_tui, App};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let has_explicit_paths = !cli.paths.is_empty();
    let mut app = App::new(
        cli.target_paths(),
        cli.parse_ecosystems(),
        cli.include_cloud,
        has_explicit_paths,
    )
    .with_intro(cli.show_intro());

    let mut filter_tokens = Vec::new();
    if let Some(ref size) = cli.min_size {
        filter_tokens.push(format!("size:>{}", size));
    }
    if let Some(days) = cli.older_than {
        filter_tokens.push(format!("days:>{}", days));
    }
    if !filter_tokens.is_empty() {
        app.search_query = filter_tokens.join(" ");
    }

    run_tui(app)?;

    Ok(())
}
