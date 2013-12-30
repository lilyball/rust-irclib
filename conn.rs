//! Management of IRC server connection

use io_error = std::io::io_error::cond;
use std::io::{TcpStream,IpAddr};
use std::io::net::addrinfo;
use std::io::net::ip::SocketAddr;
use std::{char,str,vec,uint};
use std::cmp::min;

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
    /// CTCP actions. The first arg is the destination
    IRCAction(~[u8]),
    /// CTCP commands. The first arg is the command, the second is the destination
    IRCCTCP(~[u8], ~[u8]),
    /// CTCP replies. The first arg is the command, the second is the destination
    IRCCTCPReply(~[u8], ~[u8])
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
    prefix: Option<~[u8]>,
    /// The command
    command: Command,
    /// Any arguments
    args: ~[~[u8]],
}

impl Line {
    /// Parse a line into a Line struct
    pub fn parse(mut v: &[u8]) -> Option<Line> {
        let mut prefix = None;
        if v.starts_with(bytes!(":")) {
            let idx = match v.position_elem(&(' ' as u8)) {
                None => return None,
                Some(idx) => idx
            };
            prefix = Some(v.slice(1, idx).to_owned());
            v = v.slice_from(idx+1);
        }
        let (mut command, checkCTCP) = {
            let cmd;
            match v.position_elem(&(' ' as u8)) {
                Some(0) => return None,
                None => {
                    cmd = v;
                    v = &[];
                }
                Some(idx) => {
                    cmd = v.slice_to(idx);
                    v = v.slice_from(idx+1);
                }
            }
            if cmd.len() == 3 && cmd.iter().all(|&b| b >= '0' as u8 && b <= '9' as u8) {
                (IRCCode(uint::parse_bytes(cmd, 10).unwrap()), false)
            } else if cmd.iter().all(|&b| b < 0x80 && char::is_alphabetic(b as char)) {
                let shouldCheck = cmd == bytes!("PRIVMSG") || cmd == bytes!("NOTICE");
                (IRCCmd(str::from_utf8(cmd).to_owned()), shouldCheck)
            } else {
                return None;
            }
        };
        let mut args = ~[];
        while !v.is_empty() {
            if v[0] == ':' as u8 {
                args.push(v.slice_from(1).to_owned());
                break;
            }
            let idx = match v.position_elem(&(' ' as u8)) {
                None => {
                    args.push(v.to_owned());
                    break;
                }
                Some(idx) => idx
            };
            args.push(v.slice_to(idx).to_owned());
            v = v.slice_from(idx+1);
        }
        if checkCTCP && args.len() > 1 && args.last().starts_with([0x1]) {
            let mut text = args.pop();
            if text.len() > 1 && text.ends_with([0x1]) {
                text = text.slice(1,text.len()-1).to_owned();
            } else {
                text.shift();
            }
            let dst = args[0];
            let ctcpcmd;
            match text.position_elem(&(' ' as u8)) {
                Some(idx) => {
                    ctcpcmd = text.slice_to(idx).to_owned();
                    args = ~[text.slice_from(idx+1).to_owned()];
                }
                None => {
                    ctcpcmd = text.to_owned();
                    args = ~[];
                }
            }
            match command {
                IRCCmd(~"PRIVMSG") => {
                    if bytes!("ACTION") == ctcpcmd {
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

    /// Converts into the "raw" representation :prefix cmd args
    pub fn to_raw(&self) -> ~[u8] {
        let mut cap = self.prefix.as_ref().map_default(0, |s| 1+s.len()+1);
        let mut found_space = false;
        cap += match self.command {
            IRCCmd(ref cmd) => cmd.len(),
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
            found_space = last.contains(&(' ' as u8));
            if found_space {
                cap += 1 + 1 /* : */ + last.len();
            } else {
                cap += 1 + last.len();
            }
        }
        let mut res = vec::with_capacity(cap);
        if self.prefix.is_some() {
            res.push(':' as u8);
            res.push_all(*self.prefix.as_ref().unwrap());
            res.push(' ' as u8);
        }
        match self.command {
            IRCCmd(ref cmd) => res.push_all(cmd.as_bytes()),
            IRCCode(c) => {
                uint::to_str_bytes(c, 10, |v| {
                    for _ in range(0, 3 - min(v.len(), 3)) {
                        res.push('0' as u8);
                    }
                    res.push_all(v);
                })
            }
            IRCAction(ref dst) => {
                res.push_all(bytes!("PRIVMSG "));
                res.push_all(*dst);
                res.push_all(bytes!(" :\x01ACTION"));
            }
            IRCCTCP(ref cmd, ref dst) => {
                res.push_all(bytes!("PRIVMSG "));
                res.push_all(*dst);
                res.push_all(bytes!(" :\x01"));
                res.push_all(cmd.as_slice());
            }
            IRCCTCPReply(ref cmd, ref dst) => {
                res.push_all(bytes!("NOTICE "));
                res.push_all(*dst);
                res.push_all(bytes!(" :\x01"));
                res.push_all(cmd.as_slice());
            }
        }
        if self.command.is_ctcp() {
            for arg in self.args.iter() {
                res.push(' ' as u8);
                res.push_all(*arg);
            }
            res.push(0x1);
        } else if !self.args.is_empty() {
            if self.args.len() > 1 {
                for arg in self.args.init().iter() {
                    res.push(' ' as u8);
                    res.push_all(*arg);
                }
            }
            res.push(' ' as u8);
            if found_space {
                res.push(':' as u8);
            }
            res.push_all(*self.args.last());
        }
        res
    }
}

#[cfg(test)]
mod tests {
    use super::{Line,IRCCmd,IRCCode,IRCAction,IRCCTCP,IRCCTCPReply};

    #[test]
    fn parse_line() {
        macro_rules! b(
            ($val:expr) => (bytes!($val).to_owned())
        )
        macro_rules! t(
            ($v:expr, Some($exp:expr)) => (
                t!($v, Some($exp), $v);
            );
            ($v:expr, Some($exp:expr), $res:expr) => ({
                let v = $v;
                let exp = $exp;
                let line = Line::parse(v);
                assert!(line.is_some());
                let line = line.unwrap();
                assert_eq!(line.prefix, exp.prefix);
                assert_eq!(line.command, exp.command);
                assert_eq!(line.args, exp.args);
                let line = line.to_raw();
                assert_eq!(line, $res);
            });
            ($s:expr, None) => (
                assert_eq!(Line::parse($s), None);
            )
        )
        t!(b!(":sendak.freenode.net 001 asldfkj :Welcome to the freenode Internet \
            Relay Chat Network asldfkj"),
            Some(Line{
                prefix: Some(b!("sendak.freenode.net")),
                command: IRCCode(1),
                args: ~[b!("asldfkj"),
                        b!("Welcome to the freenode Internet Relay Chat Network asldfkj")]
            }));
        t!(b!("004 asdf :This is a test"),
            Some(Line{
                prefix: None,
                command: IRCCode(4),
                args: ~[b!("asdf"), b!("This is a test")]
            }));
        t!(b!(":nick!user@host.com PRIVMSG #channel :Some message"),
            Some(Line{
                prefix: Some(b!("nick!user@host.com")),
                command: IRCCmd(~"PRIVMSG"),
                args: ~[b!("#channel"), b!("Some message")]
            }));
        t!(b!(" :sendak.freenode.net 001 asdf :Test"), None);
        t!(b!(":sendak  001 asdf :Test"), None);
        t!(b!("004"),
            Some(Line{
                prefix: None,
                command: IRCCode(4),
                args: ~[]
            }));
        t!(b!(":bob!user@host.com PRIVMSG #channel :\x01ACTION does some stuff"),
            Some(Line{
                prefix: Some(b!("bob!user@host.com")),
                command: IRCAction(b!("#channel")),
                args: ~[b!("does some stuff")]
            }),
            b!(":bob!user@host.com PRIVMSG #channel :\x01ACTION does some stuff\x01"));
        t!(b!(":bob!user@host.com PRIVMSG #channel :\x01VERSION\x01"),
            Some(Line{
                prefix: Some(b!("bob!user@host.com")),
                command: IRCCTCP(b!("VERSION"), b!("#channel")),
                args: ~[]
            }));
        t!(b!(":bob NOTICE #frobnitz :\x01RESPONSE to whatever\x01"),
            Some(Line{
                prefix: Some(b!("bob")),
                command: IRCCTCPReply(b!("RESPONSE"), b!("#frobnitz")),
                args: ~[b!("to whatever")]
            }));
        t!(b!(":bob föo"), None);
        t!(b!(":bob f23"), None);
    }
}
