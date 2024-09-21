mod app;

use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let config = app::parse_command_line()?;
    let mut terminal = ratatui::init();
    terminal.clear()?;
    let app_result = app::run(&mut terminal, config);
    ratatui::restore();
    match app_result {
        Err(error) => Err(Box::new(error)),
        Ok(()) => Ok(())
    }
}
