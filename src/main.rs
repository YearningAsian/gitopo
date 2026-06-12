mod app;
mod git;
mod ui;
mod events;
mod graph;

use anyhow::Result;
use clap::Parser;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, path::PathBuf};

use app::App;
use events::EventHandler;

#[derive(Parser, Debug)]
#[command(
    name = "gitopo",
    about = "Read-only git branch topology explorer",
    long_about = "A focused, read-only terminal UI for exploring git branch topology.\nNavigate and understand your repo's branch structure beautifully.\nZero write operations — pure visualization."
)]
struct Cli {
    /// Path to the git repository (defaults to current directory)
    #[arg(value_name = "REPO")]
    path: Option<PathBuf>,

    /// Show all branches including remotes
    #[arg(short = 'a', long)]
    all: bool,

    /// Maximum number of commits to load per branch
    #[arg(short = 'n', long, default_value = "200")]
    max_commits: usize,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let repo_path = cli
        .path
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let mut app = App::new(repo_path, cli.all, cli.max_commits)?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let event_handler = EventHandler::new(250);
    let result = run_app(&mut terminal, &mut app, event_handler);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    event_handler: EventHandler,
) -> Result<()> {
    loop {
        terminal.draw(|frame| ui::render(frame, app))?;

        if app.handle_event(event_handler.next()?)? {
            break;
        }
    }
    Ok(())
}
