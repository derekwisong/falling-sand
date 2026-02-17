use std::process::ExitCode;
use std::thread;
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
    let tick_rate = 1.0 / fps as f64;
    let mut accumulator = 0.0;
    let mut current_time = Instant::now();

    while model.state != State::Done {
        let new_time = Instant::now();
        let frame_time = new_time.duration_since(current_time).as_secs_f64();
        current_time = new_time;
        accumulator += frame_time;
        // clamp value to mitigate impact of the system performance
        accumulator = accumulator.clamp(0.0, 0.25);

        while accumulator >= tick_rate {
            let timeout = (tick_rate - frame_time).clamp(0.0, tick_rate);
            let mut message = handle_event(&model, Duration::from_secs_f64(timeout))?;
            if message.is_none() {
                message = Some(Message::Tick);
            }
            while let Some(msg) = message {
                message = update(&mut model, msg, Duration::from_secs_f64(tick_rate));
            }
            accumulator -= tick_rate;
        }

        let _alpha = accumulator / tick_rate;
        terminal.draw(|f| view(&model, f))?;

        thread::sleep(Duration::from_millis(1));
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
