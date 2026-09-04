use clap::Parser;
use degunk::cli::{run_cli, Cli};
use degunk::ui::{run_tui, App};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    if cli.should_run_tui() {
        let app = App::new(cli.target_paths(), cli.parse_ecosystems(), cli.include_cloud);
        run_tui(app)?;
    } else {
        run_cli(&cli);
    }

    Ok(())
}
