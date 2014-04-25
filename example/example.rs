/*! Example IRC bot

    This sample bot uses the library to connect to Freenode, under the nickname "rustirclib###"
    (where ### is a random number). It then joins the channel ##rustirclib, says hello, and prints
    to standard output any messages sent to the channel.

    Any incoming message of the form "rustirclib###: some message" will elicit a generic response.

    Any incoming message of the form "!rustirclib### quit" will cause the bot to shut down.
 */

#![crate_id = "github.com/kballard/rust-irclib#ircbot:0.1"]
#![crate_type = "bin"]

#![allow(deprecated_owned_vector)]

extern crate irc;
extern crate rand;

use irc::conn::{Conn, Line, Event, IRCCmd, IRCCode, IRCAction};

use std::str;
use rand::Rng;

fn main() {
    let mut opts = irc::conn::Options::new("chat.freenode.net", irc::conn::DefaultPort);

    let nick = format!("rustirclib{}", rand::task_rng().gen_range(100u, 1000u));
    opts.nick = nick.as_slice();
    match irc::conn::connect(opts, (), |c,e,_| handler(c,e)) {
        Ok(()) => println!("Exiting..."),
        Err(err) => println!("Connection error: {}", err)
    }
}

fn handler(conn: &mut Conn, event: Event) {
    match event {
        irc::conn::Connected => println!("Connected"),
        irc::conn::Disconnected => println!("Disconnected"),
        irc::conn::LineReceived(line) => {
            match line {
                Line{command: IRCCode(1), ..} => {
                    println!("Logged in");
                    // we've logged in
                    conn.join(bytes!("##rustirclib"), [])
                }
                Line{command: IRCCmd(cmd), args, prefix: prefix } => match cmd.as_slice() {
                    "JOIN" if prefix.is_some() => {
                        let prefix = prefix.unwrap();
                        if prefix.nick() != conn.me().nick() {
                            return;
                        }
                        if args.is_empty() {
                            let line = Line{command: IRCCmd(~"JOIN"), args: args, prefix: Some(prefix)};
                            println!("ERROR: Invalid JOIN message received: {}", line_desc(&line));
                            return;
                        }
                        let chan = args.move_iter().next().unwrap();
                        conn.privmsg(chan.as_slice(), bytes!("Hello"));
                        let chan = str::from_utf8_lossy(chan.as_slice());
                        println!("JOINED: {}", chan);
                    }
                    "PRIVMSG" | "NOTICE" => {
                        let (src, dst, msg) = match prefix {
                            Some(_) if args.len() == 2 => {
                                let mut args = args;
                                let (dst, msg) = (args.swap_remove(0).unwrap(),
                                                  args.move_iter().next().unwrap());
                                (prefix.as_ref().unwrap().nick(), dst, msg)
                            }
                            _ => {
                                print!("ERROR: Unexpected {} line: ", cmd);
                                let line = Line{command: IRCCmd(cmd), args: args, prefix: prefix};
                                println!("{}", line_desc(&line));
                                return;
                            }
                        };
                        let dsts = str::from_utf8_lossy(dst.as_slice());
                        let srcs = str::from_utf8_lossy(src.as_slice());
                        let msgs = str::from_utf8_lossy(msg.as_slice());
                        println!("<-- {}({}) {}: {}", cmd, dsts, srcs, msgs);
                        handle_privmsg(conn, msg.as_slice(), src, dst.as_slice())
                    }
                    _ => ()
                },
                Line{command: IRCAction(dst), args, prefix } => {
                    let (src, msg) = match prefix {
                        Some(_) if args.len() == 1 => {
                            let msg = args.move_iter().next().unwrap();
                            (prefix.as_ref().unwrap().nick(), msg)
                        }
                        _ => {
                            let line = Line{command: IRCAction(dst), args: args, prefix: prefix};
                            println!("ERROR: Unexpected ACTION line: {}", line_desc(&line));
                            return;
                        }
                    };
                    let dst = str::from_utf8_lossy(dst.as_slice());
                    let src = str::from_utf8_lossy(src.as_slice());
                    let msg = str::from_utf8_lossy(msg.as_slice());
                    println!("<-- PRIVMSG({}) {} {}", dst, src, msg);
                }
                _ => ()
            }
        }
    }
}

fn handle_privmsg(conn: &mut Conn, msg: &[u8], src: &[u8], dst: &[u8]) {
    enum MsgType<'a> {
        DirectedMessage,
        CommandMessage(&'a [u8]),
        NormalMessage
    }
    let msgtype = {
        let nick = conn.me().nick();
        if msg.starts_with(nick) && msg.slice_from(nick.len()).starts_with(bytes!(": ")) &&
        msg.len() > nick.len() + 2 {
            DirectedMessage
        } else if msg.starts_with(bytes!("!")) && msg.slice_from(1).starts_with(nick) &&
                msg.slice_from(nick.len()+1).starts_with(bytes!(" ")) {
            let args = msg.slice_from(conn.me().nick().len()+2);
            let mut argit = args.split(|&b| b == ' ' as u8).skip_while(|v| v.is_empty());
            match argit.next() {
                Some(arg) => CommandMessage(arg),
                None => CommandMessage(bytes!(""))
            }
        } else {
            NormalMessage
        }
    };
    match msgtype {
        DirectedMessage => {
            let reply = if dst == conn.me().nick() { src } else { dst };
            let msg = src + bytes!(": Hello");
            conn.privmsg(reply, msg);
            let src = str::from_utf8(conn.me().nick()).unwrap_or("(invalid utf8)");
            let reply = str::from_utf8(reply).unwrap_or("(invalid utf8)");
            let msg = str::from_utf8(msg).unwrap_or("(invalid utf8)");
            println!("--> PRIVMSG({}) {}: {}", reply, src, msg);
        }
        CommandMessage(cmd) if cmd == bytes!("quit") => {
            println!("Quitting...");
            conn.quit([]);
        }
        _ => ()
    }
}

fn line_desc(line: &Line) -> ~str {
    let raw = line.to_raw();
    str::from_utf8_lossy(raw.as_slice()).into_owned()
}
