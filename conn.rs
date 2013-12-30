//! Management of IRC server connection

use io_error = std::io::io_error::cond;
use std::io::{TcpStream,IpAddr};
use std::io::net::addrinfo;
use std::io::net::ip::SocketAddr;

/// Conn represenets a connection to a single IRC server
pub struct Conn<'a> {
    host: OptionsHost<'a>,
    tcp: TcpStream
}

/// OptionsHost allows for using an IP address or a host string
pub enum OptionsHost<'a> {
    Host(&'a str),
    Addr(IpAddr)
}

/// Options used with Conn for connecting to the server.
pub struct Options<'a> {
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
                let _guard = io_error.trap(|err| {
                    warn!("io_error resolving host address: {}", err.to_str());
                }).guard();
                match addrinfo::get_host_addresses(host) {
                    None | Some([]) => return Err("could not resolve host address"),
                    Some([x, ..]) => x
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

    let mut conn = Conn{
        host: opts.host,
        tcp: stream
    };

    conn.run();

    Ok(())
}

impl<'a> Conn<'a> {
    fn run(&mut self) {
        
    }
}

/// An IRC command
#[deriving(Eq,Clone)]
pub enum Command {
    /// An IRC command
    IRCCmd(~str),
    /// A 3-digit command code
    IRCCode(uint),
    /// CTCP actions. The string is the destination
    IRCAction(~str),
    /// CTCP commands. The first string is the command, the second is the destination
    IRCCTCP(~str, ~str),
    /// CTCP replies. The first string is the command, the second is the destination
    IRCCTCPReply(~str, ~str)
}

/// A parsed line
#[deriving(Eq,Clone)]
pub struct Line {
    /// The optional prefix
    prefix: Option<~str>,
    /// The command
    command: Command,
    /// Any arguments
    args: ~[~str],
    /// The original raw string
    raw: ~str
}

impl Line {
    pub fn parse(s: ~str) -> Option<Line> {
        let raw = s;
        // borrowck has problems
        let (prefix, command, args) = {
            let mut s = raw.as_slice();
            let mut prefix = None;
            if s.starts_with(":") {
                let idx = match s.find(' ') {
                    None => return None,
                    Some(idx) => idx
                };
                prefix = Some(s.slice(1, idx).to_owned());
                s = s.slice_from(idx+1);
            }
            let (mut command, checkCTCP) = {
                let cmd;
                match s.find(' ') {
                    Some(0) => return None,
                    None => {
                        cmd = s;
                        s = "";
                    }
                    Some(idx) => {
                        cmd = s.slice_to(idx);
                        s = s.slice_from(idx+1);
                    }
                }
                if cmd.len() == 3 && cmd.chars().all(|c| c >= '0' && c <= '9') {
                    (IRCCode(from_str(cmd).unwrap()), false)
                } else {
                    let shouldCheck = cmd == "PRIVMSG" || cmd == "NOTICE";
                    (IRCCmd(cmd.to_owned()), shouldCheck)
                }
            };
            let mut args = ~[];
            while !s.is_empty() {
                if s.starts_with(":") {
                    args.push(s.slice_from(1).to_owned());
                    break;
                }
                let idx = match s.find(' ') {
                    None => {
                        args.push(s.to_owned());
                        break;
                    }
                    Some(idx) => idx
                };
                args.push(s.slice_to(idx).to_owned());
                s = s.slice_from(idx+1);
            }
            if checkCTCP && args.len() > 1 && args.last().starts_with("\x01") {
                let mut text = args.pop();
                if text.len() > 1 && text.ends_with("\x01") {
                    text = text.slice(1,text.len()-1).to_owned();
                } else {
                    text.shift_char();
                }
                let dst = args[0];
                let mut argi = text.splitn(' ', 1).map(|s| s.to_owned());
                let ctcpcmd = argi.next().unwrap(); // splitn() should return at least 1 value
                args = argi.collect();
                match command {
                    IRCCmd(~"PRIVMSG") => {
                        if "ACTION" == ctcpcmd {
                            command = IRCAction(dst);
                        } else {
                            command = IRCCTCP(ctcpcmd, dst);
                        }
                    }
                    IRCCmd(~"NOTICE") => {
                        command = IRCCTCPReply(ctcpcmd, dst);
                    }
                    _ => unreachable!()
                }
            }
            (prefix, command, args)
        };
        Some(Line{
            prefix: prefix,
            command: command,
            args: args,
            raw: raw
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{Line,IRCCmd,IRCCode,IRCAction,IRCCTCP,IRCCTCPReply};

    #[test]
    fn parse_line() {
        assert_eq!(Line::parse(~":sendak.freenode.net 001 asldfkj :Welcome to the freenode Internet Relay Chat Network asldfkj"),
                   Some(Line{
                       prefix: Some(~"sendak.freenode.net"),
                       command: IRCCode(1),
                       args: ~[~"asldfkj", ~"Welcome to the freenode Internet Relay Chat Network asldfkj"],
                       raw: ~":sendak.freenode.net 001 asldfkj :Welcome to the freenode Internet Relay Chat Network asldfkj"
                   }));
        assert_eq!(Line::parse(~"004 asdf :This is a test"),
                   Some(Line{
                       prefix: None,
                       command: IRCCode(4),
                       args: ~[~"asdf", ~"This is a test"],
                       raw: ~"004 asdf :This is a test"
                   }));
        assert_eq!(Line::parse(~":nick!user@host.com PRIVMSG #channel :Some message"),
                   Some(Line{
                       prefix: Some(~"nick!user@host.com"),
                       command: IRCCmd(~"PRIVMSG"),
                       args: ~[~"#channel", ~"Some message"],
                       raw: ~":nick!user@host.com PRIVMSG #channel :Some message"
                   }));
        assert_eq!(Line::parse(~" :sendak.freenode.net 001 asdf :Test"), None);
        assert_eq!(Line::parse(~":sendak  001 asdf :Test"), None);
        assert_eq!(Line::parse(~"004"),
                   Some(Line{
                       prefix: None,
                       command: IRCCode(4),
                       args: ~[],
                       raw: ~"004"
                   }));
        assert_eq!(Line::parse(~":bob!user@host.com PRIVMSG #channel :\x01ACTION does some stuff"),
                   Some(Line{
                       prefix: Some(~"bob!user@host.com"),
                       command: IRCAction(~"#channel"),
                       args: ~[~"does some stuff"],
                       raw: ~":bob!user@host.com PRIVMSG #channel :\x01ACTION does some stuff"
                   }));
        assert_eq!(Line::parse(~":bob!user@host.com PRIVMSG #channel :\x01VERSION\x01"),
                   Some(Line{
                       prefix: Some(~"bob!user@host.com"),
                       command: IRCCTCP(~"VERSION", ~"#channel"),
                       args: ~[],
                       raw: ~":bob!user@host.com PRIVMSG #channel :\x01VERSION\x01"
                   }));
        assert_eq!(Line::parse(~":bob NOTICE #frobnitz :\x01RESPONSE to whatever\x01"),
                   Some(Line{
                       prefix: Some(~"bob"),
                       command: IRCCTCPReply(~"RESPONSE", ~"#frobnitz"),
                       args: ~[~"to whatever"],
                       raw: ~":bob NOTICE #frobnitz :\x01RESPONSE to whatever\x01"
                   }));
    }
}
