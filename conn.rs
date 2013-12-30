//! Management of IRC server connection

use io_error = std::io::io_error::cond;
use std::io::{TcpStream,IpAddr};
use std::io::net::addrinfo;
use std::io::net::ip::SocketAddr;
use std::str;

/// Conn represenets a connection to a single IRC server
pub struct Conn<'a> {
    host: OptionsHost<'a>,
    priv tcp: TcpStream
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

impl Command {
    /// Returns true if the command is a CTCP command
    pub fn is_ctcp(&self) -> bool {
        match *self {
            IRCAction(_) | IRCCTCP(_,_) | IRCCTCPReply(_,_) => true,
            _ => false
        }
    }
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
}

impl Line {
    /// Parse a line into a Line struct
    pub fn parse(mut s: &str) -> Option<Line> {
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
            } else if cmd.chars().all(|c| c.is_ascii() && ::std::char::is_alphabetic(c)) {
                let shouldCheck = cmd == "PRIVMSG" || cmd == "NOTICE";
                (IRCCmd(cmd.to_owned()), shouldCheck)
            } else {
                return None;
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
        Some(Line{
            prefix: prefix,
            command: command,
            args: args
        })
    }
}

impl ToStr for Line {
    fn to_str(&self) -> ~str {
        let mut cap = self.prefix.as_ref().map_default(0, |s| 1+s.len()+1);
        let mut found_space = false;
        cap += match self.command {
            IRCCmd(ref s) => s.len(),
            IRCCode(_) => 3,
            IRCAction(ref dst) => {
                "PRIVMSG".len() + 1 + dst.len() + 1 + ":\x01ACTION".len()
            }
            IRCCTCP(ref cmd, ref dst) => {
                "PRIVMSG".len() + 1 + dst.len() + 1 + 2 + cmd.len()
            }
            IRCCTCPReply(ref cmd, ref dst) => {
                "NOTICE".len() + 1 + dst.len() + 1 + 2 + cmd.len()
            }
        };
        if self.command.is_ctcp() {
            for arg in self.args.iter() {
                cap += 1 + arg.len();
            }
            cap += 1; // for the final \x01
        } else if !self.args.is_empty() {
            if self.args.len() > 1 {
                for arg in self.args.init().iter() {
                    cap += 1 + arg.len();
                }
            }
            let last = self.args.last();
            found_space = last.find(' ').is_some();
            if found_space {
                cap += 1 + 1 /* : */ + last.len();
            } else {
                cap += 1 + last.len();
            }
        }
        let mut res = str::with_capacity(cap);
        if self.prefix.is_some() {
            res.push_char(':');
            res.push_str(*self.prefix.as_ref().unwrap());
            res.push_char(' ');
        }
        match self.command {
            IRCCmd(ref s) => res.push_str(*s),
            IRCCode(c) => {
                res.push_str(format!("{:03u}", c));
            }
            IRCAction(ref dst) => {
                res.push_str("PRIVMSG ");
                res.push_str(*dst);
                res.push_str(" :\x01ACTION");
            }
            IRCCTCP(ref cmd, ref dst) => {
                res.push_str("PRIVMSG ");
                res.push_str(*dst);
                res.push_str(" :\x01");
                res.push_str(*cmd);
            }
            IRCCTCPReply(ref cmd, ref dst) => {
                res.push_str("NOTICE ");
                res.push_str(*dst);
                res.push_str(" :\x01");
                res.push_str(*cmd);
            }
        }
        if self.command.is_ctcp() {
            for arg in self.args.iter() {
                res.push_char(' ');
                res.push_str(*arg);
            }
            res.push_char('\x01');
        } else if !self.args.is_empty() {
            if self.args.len() > 1 {
                for arg in self.args.init().iter() {
                    res.push_char(' ');
                    res.push_str(*arg);
                }
            }
            res.push_char(' ');
            if found_space {
                res.push_char(':');
            }
            res.push_str(*self.args.last());
        }
        res
    }
}

#[cfg(test)]
mod tests {
    use super::{Line,IRCCmd,IRCCode,IRCAction,IRCCTCP,IRCCTCPReply};

    #[test]
    fn parse_line() {
        macro_rules! t(
            ($s:expr, Some($exp:expr)) => ({
                t!($s, Some($exp), $s);
            });
            ($s:expr, Some($exp:expr), $res:expr) => ({
                let s = $s;
                let line = Line::parse(s);
                assert_eq!(line, Some($exp));
                let line = line.unwrap().to_str();
                assert_eq!(line.as_slice(), $res);
            });
            ($s:expr, None) => (
                assert_eq!(Line::parse($s), None);
            )
        )
        t!(":sendak.freenode.net 001 asldfkj :Welcome to the freenode Internet \
            Relay Chat Network asldfkj",
            Some(Line{
                prefix: Some(~"sendak.freenode.net"),
                command: IRCCode(1),
                args: ~[~"asldfkj",
                        ~"Welcome to the freenode Internet Relay Chat Network asldfkj"],
            }));
        t!("004 asdf :This is a test",
            Some(Line{
                prefix: None,
                command: IRCCode(4),
                args: ~[~"asdf", ~"This is a test"],
            }));
        t!(":nick!user@host.com PRIVMSG #channel :Some message",
            Some(Line{
                prefix: Some(~"nick!user@host.com"),
                command: IRCCmd(~"PRIVMSG"),
                args: ~[~"#channel", ~"Some message"],
            }));
        t!(" :sendak.freenode.net 001 asdf :Test", None);
        t!(":sendak  001 asdf :Test", None);
        t!("004",
            Some(Line{
                prefix: None,
                command: IRCCode(4),
                args: ~[],
            }));
        t!(":bob!user@host.com PRIVMSG #channel :\x01ACTION does some stuff",
            Some(Line{
                prefix: Some(~"bob!user@host.com"),
                command: IRCAction(~"#channel"),
                args: ~[~"does some stuff"],
            }),
            ":bob!user@host.com PRIVMSG #channel :\x01ACTION does some stuff\x01");
        t!(":bob!user@host.com PRIVMSG #channel :\x01VERSION\x01",
            Some(Line{
                prefix: Some(~"bob!user@host.com"),
                command: IRCCTCP(~"VERSION", ~"#channel"),
                args: ~[],
            }));
        t!(":bob NOTICE #frobnitz :\x01RESPONSE to whatever\x01",
            Some(Line{
                prefix: Some(~"bob"),
                command: IRCCTCPReply(~"RESPONSE", ~"#frobnitz"),
                args: ~[~"to whatever"],
            }));
        t!(":bob föo", None);
        t!(":bob f23", None);
    }
}
