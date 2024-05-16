mod client;
// Alright boy - step 1,
//
// build something you can type a prompt into
//
// step 2
//
// add chatgpt-api
//
// step 3
//
// success

use crate::client::GptClient;
use client::Output;
use markdown::mdast::Node;
use pulldown_cmark_mdcat::resources::NoopResourceHandler;
use pulldown_cmark_mdcat::{Environment, Settings, TerminalSize};
use std::env;
use std::io::stdout;
use std::path::Path;
use std::{
    error::Error,
    io::{stdin, Write},
};
use tokio::{spawn, sync::mpsc};

use markdown::{to_mdast, ParseOptions};
use pulldown_cmark::{Event as MdEvent, Options, Parser};
use pulldown_cmark_mdcat::{push_tty, terminal::TerminalProgram};
use syntect::parsing::SyntaxSet;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Check API-Key
    if let Err(e) = env::var("OPENAI_KEY") {
        eprintln!("Failed to get OPENAI_KEY: {e}");
        std::process::exit(1);
    }

    // Initialize terminal
    let args: Vec<String> = env::args().collect();

    // Ok, focus on the gpt-part for a short time
    let mut input = String::new();

    if args.len() > 1 {
        for word in args.iter().skip(1) {
            input.push_str(word);
            input.push(' ');
        }
        input.pop();
    } else {
        let std_input = stdin();
        std_input.read_line(&mut input)?;
        println!("");
    };

    let (input_tx, input_rx) = mpsc::channel(16);
    let (output_tx, mut output_rx) = mpsc::channel(16);

    // Create a new client and spawn an event stream
    let client = GptClient::new();
    let handle = spawn(client.event_stream(input_rx, output_tx));
    input_tx.send(input).await?;

    // In the meantime, initialize the terminal output
    let tp = TerminalProgram::detect();
    let ss = SyntaxSet::load_defaults_newlines();
    let mut terminal_size = TerminalSize::detect().unwrap();
    terminal_size.columns = terminal_size.columns / 2;
    let settings = Settings {
        terminal_capabilities: tp.capabilities(),
        terminal_size,
        syntax_set: &ss,
        theme: pulldown_cmark_mdcat::Theme::default(),
    };
    // Why the fuck do we need this ?
    let current_path = Path::new(".").canonicalize()?;
    let environment = Environment::for_local_directory(&current_path)?;
    let rs_handler = NoopResourceHandler;

    // NOTE: We could use MAX_TOKENS to initialize the answer string correctly,
    // however 10k should be enough for most questions.
    let mut full_answer = String::with_capacity(10_000);

    loop {
        // Initialize last-children-len with 1, because we only print after having at least two nodes.
        let mut last_children_len = 1;
        let mut chunk_answer = String::with_capacity(1_000);
        // And await events from gpt-client
        while let Some(output) = output_rx.recv().await {
            match output {
                Output::Data(answer) => {
                    full_answer.push_str(&answer);
                    match to_mdast(&full_answer, &ParseOptions::default()) {
                        Ok(Node::Root(root)) => {
                            if root.children.len() > last_children_len {
                                // We are super sneaky, and just print each chunk,
                                // whenever there is a new node in the root tree of our document.
                                let parser = Parser::new_ext(&chunk_answer, Options::all());
                                push_tty(
                                    &settings,
                                    &environment,
                                    &rs_handler,
                                    &mut stdout(),
                                    parser,
                                )?;
                                chunk_answer.clear(); // reset chunk
                                last_children_len = root.children.len();
                            }
                        }
                        Err(e) => {
                            println!("ERROR: Failed to parse - {e}");
                        }
                        _ => {}
                    }
                    chunk_answer.push_str(&answer);
                }
                Output::End => {
                    let parser = Parser::new_ext(&chunk_answer, Options::all());
                    push_tty(&settings, &environment, &rs_handler, &mut stdout(), parser)?;
                    chunk_answer.clear(); // reset chunk
                                          // println!("{full_answer}");
                    full_answer.clear();
                    break;
                }
            }
        }
        println!("\n--- Input: ");
        // Let's take another input
        let mut input = String::new();
        stdin().read_line(&mut input)?;

        // add some commands here
        if input.starts_with('/') {
            match input.to_lowercase().as_str() {
                "/exit" => break,
                _other => (),
            }
        }

        println!("--- ChatGPT: ");
        input_tx.send(input).await?;
    }
    // Drop the handles here, so that the handle can return properly
    drop(input_tx);
    drop(output_rx);

    handle.await??;
    Ok(())
}
