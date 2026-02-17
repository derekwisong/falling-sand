use std::process::ExitCode;
use std::time::{Duration, Instant};

use falling_sand::{Message, Model, State};

use ratatui::{
    DefaultTerminal, Frame,
    crossterm::event::{self, Event, KeyCode},
};

fn view(model: &Model, frame: &mut Frame) {
    frame.render_widget(model, frame.area());
}

fn update(model: &mut Model, msg: Message, delta: Duration) -> Option<Message> {
    model.update(msg, delta)
}

fn handle_key(key: event::KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Char('q') => Some(Message::Quit),
        _ => None,
    }
}

fn handle_event(_: &Model, timeout: Duration) -> std::io::Result<Option<Message>> {
    let e = event::poll(timeout)?;
    if e {
        match event::read()? {
            Event::Key(key) if key.kind == event::KeyEventKind::Press => Ok(handle_key(key)),
            Event::Resize(x, y) => Ok(Some(Message::Resize(x as usize, y as usize))),
            _ => Ok(None),
        }
    } else {
        Ok(None)
    }
}

fn app(terminal: &mut DefaultTerminal) -> std::io::Result<()> {
    let mut model = Model::default();
    let terminal_dims = terminal.get_frame().area();
    model.resize(terminal_dims.width as usize, terminal_dims.height as usize);
    let fps = 60;
    let tick_rate = Duration::from_secs_f64(1.0 / fps as f64);
    let mut last_tick = Instant::now();

    while model.state != State::Done {
        terminal.draw(|f| view(&model, f))?;
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or(Duration::from_secs(0));
        let mut message = handle_event(&model, timeout)?;
        if last_tick.elapsed() >= tick_rate {
            if message.is_none() {
                message = Some(Message::Tick);
            }
            last_tick = Instant::now();
        }

        while let Some(msg) = message {
            message = update(&mut model, msg, tick_rate);
        }
    }

    Ok(())
}

fn main() -> ExitCode {
    match ratatui::run(app) {
        Ok(_) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("Error: {}", err);
            ExitCode::FAILURE
        }
    }
}
