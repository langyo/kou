use clap::Parser;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "kou", about = "Virtual terminal automation")]
struct Cli {
    #[command(subcommand)]
    cmd: Option<Command>,
}

#[derive(clap::Subcommand)]
enum Command {
    /// Launch a command in a virtual terminal.
    Launch {
        command: String,
        #[arg(short, long, default_value = "80")]
        cols: u16,
        #[arg(short, long, default_value = "24")]
        rows: u16,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    match cli.cmd {
        Some(Command::Launch {
            command,
            cols,
            rows,
        }) => {
            let mgr = kou::VttyManager::new();
            let id = mgr.launch(&command, None, cols, rows).await?;
            println!("Session: {}", id);

            // Simple REPL
            loop {
                use std::io::{self, BufRead, Write};
                print!("> ");
                io::stdout().flush()?;

                let mut line = String::new();
                io::stdin().lock().read_line(&mut line)?;
                let line = line.trim();

                match line {
                    "exit" | "quit" => break,
                    "screen" => {
                        let text = mgr.screenshot(&id).await?;
                        println!("{}", text);
                    }
                    _ => {
                        mgr.send_text(&id, &format!("{}\n", line)).await?;
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                        let text = mgr.screenshot(&id).await?;
                        println!("{}", text);
                    }
                }
            }

            mgr.kill(&id).await;
        }
        None => {
            eprintln!("Usage: kou launch <command> [--cols 80] [--rows 24]");
        }
    }

    Ok(())
}
