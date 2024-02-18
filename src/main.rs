mod client;
// Alright boy - step 1,
//
// build something you can type a promt into
//
// step 2
//
// add chatgpt-api
//
// step 3
//
// success

use std::{
    error::Error,
    io::{stdin, stdout, Write},
};

use tokio::{spawn, sync::mpsc};

use crate::client::GptClient;
use std::env;

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

    let mut client = GptClient::new();

    let mut input = String::new();

    if args.len() > 1 {
        for word in args.iter().skip(1) {
            input.push_str(word);
            input.push(' ');
        }
        input.pop();
    } else {
        let std_input = stdin();
        println!("");
        std_input.read_line(&mut input)?;
        println!("");
    };
    // let result = client.make_request(input).await?;

    let (input_tx, input_rx) = mpsc::channel(16);
    let (output_tx, mut output_rx) = mpsc::channel(16);

    let handle = spawn(client.event_stream(input_rx, output_tx));
    input_tx.send(input).await?;

    while let Some(answer) = output_rx.recv().await {
        print!("{}", answer);
        std::io::stdout().flush()?;
    }

    handle.await??;
    Ok(())
}
