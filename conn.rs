//! Management of IRC server connection

use sdt::io::io_error;
use std::io::TcpStream;
use std::io::net::addrinfo;
use std::io::net::ip::SocketAddr;
use std::send_str::SendStr;

/// Conn represenets a connection to a single IRC server
pub struct<'a> Conn<'a> {
    host: OptionsHost<'a>,
    priv incoming: Port<~[u8]>,
    priv outgoing: Chan<~[u8]>
}

/// OptionsHost allows for using an IP address or a host string
pub enum<'a> OptionsHost<'a> {
    Host(&'a str),
    Addr(std::io::IpAddr)
}

/// Options used with Conn for connecting to the server.
pub struct<'a> Options<'a> {
    host: OptionsHost<'a>,
    port: u16
}

impl<'a> Options<'a> {
    /// Returns a new Options struct with default values
    pub fn new(host: &'a str, port: u16) -> Options<'a> {
        #[inline];
        Options {
            host: Host(host),
            port: port
        }
    }
}

pub static DefaultPort: u16 = 6667;

/// Connects to the remote server. This method will not return until the connection
/// is terminated. Returns Ok(()) after connection termination if the connection was
/// established successfully, or Err(&str) if the connection could not be established in the
/// first place.
///
/// # Failure
///
/// Raises the `io_error` condition if an IO error happens at any point after the connection
/// is established.
pub fn connect(opts: Options) -> Result<(),&'static str> {
    let addr = {
        match opts.host {
            Addr(x) => x,
            Host(host) => {
                let guard = io_error.trap(|err| {
                    warn!("io_error resolving host address: {}", err.to_str());
                }).guard();
                match addrinfo::get_host_addresses(host) {
                    None | Some(~[]) => return Err("could not resolve host address"),
                    Some(~[x, ..]) => x
                }
            }
        }
    };
    let addr = SocketAddr{ ip: addr, port: opts.port };

    let stream = io_error.trap(|err| {
        warn!("io_error connecting to server: {}", err.to_str());
    }).inside(|| {
        // I don't know if ::connect() can throw io_error, but better safe than sorry
        TcpStream::connect(addr)
    });
    let stream = match stream {
        None => return Err("could not connect to server"),
        Some(tcp) => tcp
    };

    let host = opts.host;
    do green::run {
        let (outgoingport, outgoingchan) = Chan::new();
        let (incomingport, incomingchan) = Chan::new();
        let mut task = std::task::task();
        task.name("libirc I/O");
        do task.spawn {
            handle_io(tcp, outgoingport, incomingchan);
        }

        let conn = Conn{
            host: host,
            incoming: incomingport,
            outgoing: outgoingchan,
        }

        conn.start();
    }
    Ok(())
}

enum IOMessage {
    Msg(~[u8]),
    Quit
}

/// Handles I/O on `tcp`, sending each read newline-delimited line to the `incoming` port and
/// writing each message from `outgoing` as a newline-terminated line.
///
/// If the `Quit` message is received on `outgoing`, sends the "QUIT" IRC command to the server,
/// waits for the server to terminate the connection, sends the `Quit` message to `incoming` and
/// returns.
///
/// If the IO connection is closed or an `io_error` is raised, this function returns.
fn handle_io(tcp: TcpStream, outgoing: Port<IOMessage>, incoming: Chan<IOMessage>) {
    do green::run {

    }
}

impl Conn {
    fn start(&self) {

    }
}
