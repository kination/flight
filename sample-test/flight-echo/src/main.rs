use clap::{Parser, Subcommand};
use flight::{Context, FlightError, Mode, Program};

#[derive(Parser)]
#[command(about = "Flight echo example - server/client data exchange")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Server {
        #[arg(short, long, default_value = "9000")]
        port: u16,

        #[arg(short, long)]
        iface: Option<String>,
    },
    Client {
        #[arg(short, long, default_value = "127.0.0.1:9000")]
        server: String,

        #[arg(short, long, default_value = "Hello")]
        message: String,
    },
}

fn main() -> Result<(), FlightError> {
    let cli = Cli::parse();

    match cli.command {
        Command::Server { port, iface } => run_server(port, iface),
        Command::Client { server, message } => run_client(&server, &message),
    }
}

fn run_server(port: u16, iface: Option<String>) -> Result<(), FlightError> {
    let mut ctx = Context::new(Mode::Echo);
    ctx.set_port(port);

    if let Some(iface_name) = iface {
        ctx.set_iface(&iface_name);
    }

    let mut prog = Program::with_context(ctx)?;
    prog.attach()?;

    println!("[server] listening on {}", prog.local_addr()?);
    println!("[server] Echo mode: all received data will be sent back.");
    println!();

    let mut buf = [0u8; 1024];
    loop {
        let (from, n) = prog.recv(&mut buf)?;
        let data = &buf[..n];
        println!("[server] {} -> {} bytes: {:?}", from, n, String::from_utf8_lossy(data));
        println!("[server] echoed back to {from}");
        println!("[server] {}", prog.stats());
    }
}

fn run_client(server: &str, message: &str) -> Result<(), FlightError> {
    let mut ctx = Context::new(Mode::Echo);
    ctx.set_port(0);

    let mut prog = Program::with_context(ctx)?;
    prog.attach()?;

    let dest: std::net::SocketAddr = server
        .parse()
        .map_err(|e| FlightError::Send(format!("invalid address '{server}': {e}")))?;

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
