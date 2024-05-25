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
use client::{Input, Output, UseContext};
use markdown::mdast::Node;
use pulldown_cmark_mdcat::resources::NoopResourceHandler;
use pulldown_cmark_mdcat::{Environment, Settings, TerminalSize};
use std::env;
use std::io::stdout;
use std::path::Path;
use std::{error::Error, io::stdin};
use tokio::sync::mpsc::Sender;
use tokio::{spawn, sync::mpsc};

use markdown::{to_mdast, ParseOptions};
use pulldown_cmark::{Options, Parser};
use pulldown_cmark_mdcat::{push_tty, terminal::TerminalProgram};
use syntect::parsing::SyntaxSet;

struct MdPrinter {
    tp: TerminalProgram,
    ss: SyntaxSet,
    environment: Environment,
    rs_handler: NoopResourceHandler,
}

impl MdPrinter {
    pub fn new() -> Result<MdPrinter, Box<dyn Error>> {
        // Initialize parser (lots of boilerplate)
        let tp = TerminalProgram::detect();
        let ss = SyntaxSet::load_defaults_newlines();
        let current_path = Path::new(".").canonicalize()?;
        let environment = Environment::for_local_directory(&current_path)?;
        let rs_handler = NoopResourceHandler;
        Ok(MdPrinter {
            tp,
            ss,
            environment,
            rs_handler,
        })
    }

    pub fn print(&self, input: impl AsRef<str>) -> Result<(), Box<dyn Error>> {
        let mut terminal_size = TerminalSize::detect().unwrap();
        terminal_size.columns = if terminal_size.columns / 2 < 77 {
            3 * terminal_size.columns / 4
        } else {
            terminal_size.columns / 2
        };
        let settings = Settings {
            terminal_capabilities: self.tp.capabilities(),
            terminal_size,
            syntax_set: &self.ss,
            theme: pulldown_cmark_mdcat::Theme::default(),
        };
        let parser = Parser::new_ext(input.as_ref(), Options::all());
        push_tty(
            &settings,
            &self.environment,
            &self.rs_handler,
            &mut stdout(),
            parser,
        )?;
        Ok(())
    }
}

async fn get_user_input() -> Result<String, Box<dyn Error>> {
    let mut input = String::with_capacity(1_000);
    let std_input = stdin();
    std_input.read_line(&mut input)?;
    Ok(input)
}

async fn process_input(input: &str, input_tx: &Sender<Input>) -> Result<bool, Box<dyn Error>> {
    let print_help = || {
        println!("-- Basic commands:");
        println!("- Showing this help screen - '/help' or '/h'");
        println!("- Clearing conversation    - '/clear' or '/c' or '/new' or '/n'");
        println!("- Quit program             - '/quit' or '/q' or '/exit' or '/stop'");
        println!("");
        println!("-- Change context:");
        println!("- Basic (standard chatgpt-context)         - '/basic' or '/b'");
        println!("- Short (shorter, more direct answers)     - '/short' or '/s'");
        println!("- Programming (fine tuned for programmers) - '/programming' or '/prog' or '/p'");
        println!("");
        println!("You can set the default context via environment variable RGPT_CONTEXT='basic'");
    };
    // add some commands here
    if input.starts_with('/') {
        match input.trim().to_lowercase().as_str() {
            "/exit" | "/quit" | "/q" | "/stop" => std::process::exit(0),
            "/help" | "/h" => print_help(),
            "/programming" | "/prog" | "/p" => {
                input_tx
                    .send(Input::Context(UseContext::Programming))
                    .await?
            }
            "/short" | "/s" => input_tx.send(Input::Context(UseContext::Short)).await?,
            "/basic" | "/b" => input_tx.send(Input::Context(UseContext::Basic)).await?,
            "/clear" | "/c" | "/new" | "/n" => input_tx.send(Input::Clear).await?,
            _other => println!("--- System: Invalid input."),
        }
        return Ok(true);
    }
    input_tx.send(Input::Text(input.to_string())).await?;
    Ok(false)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Check API-Key
    if let Err(e) = env::var("OPENAI_KEY") {
        eprintln!("Failed to get OPENAI_KEY: {e}");
        std::process::exit(1);
    }

    // Create a new client and spawn an event stream
    let client = GptClient::new();
    let (input_tx, input_rx) = mpsc::channel(16);
    let (output_tx, mut output_rx) = mpsc::channel(16);
    let _handle = spawn(client.event_stream(input_rx, output_tx));

    // Create markdown printer
    let md = MdPrinter::new()?;

    // Parse input (if any)
    let args: Vec<String> = env::args().collect();
    let mut input = String::new();
    if args.len() > 1 {
        for word in args.iter().skip(1) {
            input.push_str(word);
            input.push(' ');
        }
        input.pop();
    } else {
        md.print("# Input")?;
        input = get_user_input().await?;
    };
    while process_input(&input, &input_tx).await? {
        input = get_user_input().await?;
    }

    // NOTE: We could use MAX_TOKENS to initialize the answer string correctly,
    // however 10k should be enough for most questions.
    let mut full_answer = String::with_capacity(10_000);

    loop {
        // Prepare the answer box
        println!("");
        md.print("# ChatGPT")?;

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
                                md.print(&chunk_answer)?;
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
                    md.print(&chunk_answer)?;
                    chunk_answer.clear();
                    full_answer.clear();
                    break;
                }
            }
        }
        println!("");
        md.print("# Input")?;
        // Let's take another input
        input = get_user_input().await?;
        while process_input(&input, &input_tx).await? {
            input = get_user_input().await?;
        }
    }
    // Drop the handles here, so that the handle can return properly
    // drop(input_tx);
    // drop(output_rx);

    // handle.await??;
    // Ok(())
}
