use clap::{Parser, Subcommand};
use std::fs;
use xpresso::{Context, FlightError, Mode, Program};

#[derive(Parser)]
#[command(about = "Flight echo example - server/client data exchange")]
struct Cli {
    #[arg(long)]
    config: Option<String>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Server {
        #[arg(short, long)]
        port: Option<u16>,

        #[arg(short, long)]
        iface: Option<String>,
    },
    Client {
        #[arg(short, long)]
        server: Option<String>,

        #[arg(short, long, default_value = "Hello")]
        message: String,
    },
}

fn main() -> Result<(), FlightError> {
    let cli = Cli::parse();

    let mut ctx = if let Some(path) = &cli.config {
        let content = fs::read_to_string(path).map_err(|e| FlightError::Io(e))?;
        toml::from_str(&content).map_err(|e| FlightError::Bind(format!("config error: {e}")))?
    } else {
        Context::new(Mode::Echo)
    };

    match cli.command {
        Command::Server { port, iface } => {
            if let Some(p) = port {
                ctx.set_port(p);
            } else if ctx.port() == 0 {
                ctx.set_port(9000); // Default if not in config nor CLI
            }

            if let Some(i) = iface {
                ctx.set_iface(&i);
            }

            run_server(ctx)
        }
        Command::Client { server, message } => {
            // Client mode doesn't need to listen necessarily, but Context is used for Program.
            // Client uses `ctx` for `Program`, typically port 0 (ephemeral).
            // `server` arg is destination.
            run_client(ctx, server.as_deref(), &message)
        }
    }
}

fn run_server(ctx: Context) -> Result<(), FlightError> {
    let mut prog = Program::with_context(ctx)?;
    prog.attach()?;

    println!("[server] listening on {}", prog.local_addr()?);
    println!("[server] Echo mode: all received data will be sent back.");
    println!();

    let mut buf = [0u8; 1024];
    loop {
        let (from, n) = prog.recv(&mut buf)?;
        let data = &buf[..n];
        println!(
            "[server] {} -> {} bytes: {:?}",
            from,
            n,
            String::from_utf8_lossy(data)
        );
        println!("[server] echoed back to {from}");
        println!("[server] {}", prog.stats());
    }
}

fn run_client(ctx: Context, server: Option<&str>, message: &str) -> Result<(), FlightError> {
    // If not specified, client binds to ephemeral port (0)
    if ctx.port() == 0 {
        // keep 0
    }

    let mut prog = Program::with_context(ctx)?;
    prog.attach()?;

    let server_addr = server.unwrap_or("127.0.0.1:9000");
    let dest: std::net::SocketAddr = server_addr
        .parse()
        .map_err(|e| FlightError::Send(format!("invalid address '{server_addr}': {e}")))?;

    println!("[client] sending to {dest}: {message:?}");
    let sent = prog.send(&dest, message.as_bytes())?;
    println!("[client] sent {sent} bytes");

    let mut buf = [0u8; 1024];
    let (from, n) = prog.recv(&mut buf)?;
    let reply = String::from_utf8_lossy(&buf[..n]);
    println!("[client] reply from {from}: {reply:?}");

    println!("[client] {}", prog.stats());

    Ok(())
}
