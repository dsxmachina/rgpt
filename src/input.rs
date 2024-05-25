use std::io::{stdout, Write};

use crossterm::{
    cursor::{MoveLeft, MoveToNextLine},
    event::{Event, EventStream, KeyCode, KeyEvent, KeyModifiers},
    queue,
    style::Print,
    terminal::{disable_raw_mode, enable_raw_mode, size, Clear, ClearType, ScrollDown, ScrollUp},
    QueueableCommand,
};
use futures::{FutureExt, StreamExt};

fn handle_key_event(
    key_event: KeyEvent,
    input_buffer: &mut String,
) -> Result<bool, std::io::Error> {
    let mut stdout = stdout();
    // Ctrl+C or Ctrl+D makes the program exit
    if key_event.modifiers.contains(KeyModifiers::CONTROL) {
        match key_event.code {
            KeyCode::Char('d') | KeyCode::Char('c') => {
                // Cleanup
                disable_raw_mode()?;
                std::process::exit(0);
            }
            _ => (),
        }
        return Ok(false);
    }
    if let KeyCode::Enter = key_event.code {
        if key_event.modifiers.contains(KeyModifiers::SHIFT) {
            stdout.queue(MoveToNextLine(1))?;
            input_buffer.push('\n')
        } else {
            return Ok(true);
        }
    }
    match key_event.code {
        crossterm::event::KeyCode::Backspace => {
            stdout.queue(MoveLeft(1))?;
            stdout.queue(Clear(ClearType::UntilNewLine))?;
            input_buffer.pop();
        }
        crossterm::event::KeyCode::Char(c) => {
            stdout.queue(Print(c))?;
            input_buffer.push(c);
        }
        // Exit with escape ?
        // crossterm::event::KeyCode::Esc => std::process::exit(0),
        _ => (),
    }
    stdout.flush()?;
    Ok(false)
}

pub async fn get_user_input() -> Result<String, std::io::Error> {
    let mut input = String::with_capacity(1_000);

    // Enable raw-mode and capture all keyboard-input
    enable_raw_mode()?;

    let mut event_stream = EventStream::new();

    while let Some(event) = event_stream.next().fuse().await {
        match event? {
            Event::Key(key_event) => {
                if handle_key_event(key_event, &mut input)? {
                    break;
                }
            }
            Event::Paste(string) => {
                input.push_str(&string);
            }
            _ => continue,
        };
    }

    // Cleanup
    disable_raw_mode()?;
    Ok(input)
}
