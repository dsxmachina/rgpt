use crossterm::{
    cursor,
    event::{DisableMouseCapture, Event, EventStream, KeyCode, KeyEvent, KeyModifiers},
    queue,
    style::{self, Stylize},
    terminal::{
        disable_raw_mode, enable_raw_mode, size, Clear, ClearType, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
    QueueableCommand,
};

use futures::{FutureExt, StreamExt};

use crate::client::GptClient;

type Result<T> = std::result::Result<T, Error>;

struct InputBuffer {
    lines: Vec<String>,
}

impl InputBuffer {
    pub fn new() -> Self {
        let mut lines = Vec::new();
        lines.push(String::new());
        InputBuffer { lines }
    }

    pub fn newline(&mut self) {
        self.lines.push(String::new());
    }

    pub fn push(&mut self, c: char) {
        if let Some(s) = self.lines.last_mut() {
            s.push(c);
        }
    }

    pub fn pop(&mut self) {
        if let Some(s) = self.lines.last_mut() {
            s.pop();
        }
    }

    pub fn lines_numbered(&self) -> impl Iterator<Item = (usize, &String)> {
        self.lines.iter().enumerate()
    }

    pub fn lines(&self) -> impl Iterator<Item = &String> {
        self.lines.iter()
    }
}

fn handle_input(key_event: KeyEvent, input_buffer: &mut InputBuffer) -> Result<bool> {
    if key_event.modifiers.contains(KeyModifiers::CONTROL) {
        match key_event.code {
            KeyCode::Char('d') => return Ok(true),
            _ => (),
        }
        return Ok(false);
    }
    if key_event.modifiers.contains(KeyModifiers::SHIFT) {
        match key_event.code {
            KeyCode::Enter => input_buffer.newline(),
            _ => (),
        }
        return Ok(false);
    } else {
        match key_event.code {
            crossterm::event::KeyCode::Backspace => {
                input_buffer.pop();
            }
            crossterm::event::KeyCode::Enter => input_buffer.newline(),
            crossterm::event::KeyCode::Char(c) => {
                input_buffer.push(c);
            }
            crossterm::event::KeyCode::Esc => return Ok(true),
            _ => (),
        }
    }

    if key_event.modifiers.contains(KeyModifiers::NONE) {}

    Ok(false)
}

fn print(prompt: &InputBuffer) -> Result<()> {
    let mut stdout = stdout();
    let (_sx, sy) = size()?;
    queue!(
        stdout,
        cursor::MoveTo(1, 0),
        style::PrintStyledContent(
            "Hello. How can I help you ?"
                .to_string()
                .dark_green()
                .bold()
        ),
    );
    for (i, line) in prompt.lines_numbered() {
        // TODO: Handle prompt bigger than sy
        if i > sy as usize {
            break;
        }
        queue!(
            stdout,
            cursor::MoveTo(1, i as u16 + 2),
            Clear(ClearType::CurrentLine),
            // style::PrintStyledContent(prompt.dark_green().bold()),
            style::Print(line),
            // style::PrintStyledContent(prefix.to_string().dark_blue().bold()),
            // style::PrintStyledContent(suffix.to_string().white().bold()),
        )?;
    }
    stdout.flush()?;
    Ok(())
}

async fn event_loop() -> Result<()> {
    let mut event_stream = EventStream::new();

    let mut input_buffer = InputBuffer::new();

    while let Some(event) = event_stream.next().fuse().await {
        let key_event = match event? {
            Event::Key(key_event) => key_event,
            _ => continue,
        };

        if handle_input(key_event, &mut input_buffer)? {
            break;
        }

        print(&input_buffer)?;

        // Print input buffer
    }

    Ok(())
}
