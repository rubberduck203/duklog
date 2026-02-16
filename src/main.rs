#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

use std::io::{self, stdout};

use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use duklog::storage::LogManager;
use duklog::tui::App;

#[cfg_attr(coverage_nightly, coverage(off))]
#[mutants::skip]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = restore_terminal();
        original_hook(info);
    }));

    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let manager = LogManager::new()?;
    let mut app = App::new(manager);
    let result = app.run(&mut terminal);

    restore_terminal()?;
    result.map_err(Into::into)
}

fn restore_terminal() -> Result<(), io::Error> {
    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)
}
