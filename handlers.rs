//! Built-in IRC message handlers

use conn::{IRCCode, IRCCmd, Conn, Line};

pub fn handle_line<P>(conn: &mut Conn<P>, line: &Line) {
    if !conn.logged_in {
        match line.command {
            IRCCode(001) => handshake::RPL_WELCOME(conn, line),
            IRCCode(433) => handshake::ERR_NICKNAMEINUSE(conn, line),
            IRCCode(432) => handshake::ERR_ERRONEUSNICKNAME(conn, line),
            IRCCode(436) => handshake::ERR_NICKCOLLISION(conn, line),
            IRCCode(437) => handshake::ERR_UNAVAILRESOURCE(conn, line),
            IRCCmd(~"PING") => normal::PING(conn, line),
            _ => ()
        }
    } else {
        match line.command {
            IRCCmd(~"PING") => normal::PING(conn, line),
            IRCCmd(~"NICK") => normal::NICK(conn, line),
            _ => ()
        }
    }
}

mod handshake {
    use conn::{Conn, Line};

    // 001
    pub fn RPL_WELCOME<P>(conn: &mut Conn<P>, line: &Line) {
        conn.logged_in = true;
        if !line.args.is_empty() {
            conn.user = conn.user.with_nick(line.args[0]);
        }
    }

    // 433
    pub fn ERR_NICKNAMEINUSE<P>(conn: &mut Conn<P>, line: &Line) {
        if !line.args.is_empty() {
            let nick = line.args[0].as_slice();
            if nick == conn.user.nick() {
                conn.set_nick(nick + bytes!("_"));
                return;
            }
        }
        // nick was truncated? Fall back to generic _-replacement behavior
        bad_nick(conn, line);
    }

    // 432
    pub fn ERR_ERRONEUSNICKNAME<P>(conn: &mut Conn<P>, line: &Line) {
        bad_nick(conn, line);
    }

    // 436
    pub fn ERR_NICKCOLLISION<P>(conn: &mut Conn<P>, line: &Line) {
        bad_nick(conn, line);
    }

    // 437
    pub fn ERR_UNAVAILRESOURCE<P>(conn: &mut Conn<P>, line: &Line) {
        bad_nick(conn, line);
    }

    fn bad_nick<P>(conn: &mut Conn<P>, line: &Line) {
        let mut nick;
        if !line.args.is_empty() {
            nick = line.args[0].clone();
        } else {
            nick = conn.user.nick().to_owned();
        }

        let mut modified = false;
        for b in nick.mut_rev_iter() {
            if *b != '_' as u8 {
                *b = '_' as u8;
                modified = true;
                break;
            }
        }
        if modified {
            conn.set_nick(nick);
        } else {
            conn.quit([]);
        }
    }
}

mod normal {
    use conn::{IRCCmd, Conn, Line};

    pub fn PING<P>(conn: &mut Conn<P>, line: &Line) {
        conn.send_command(IRCCmd(~"PONG"), line.args, false);
    }

    pub fn NICK<P>(conn: &mut Conn<P>, line: &Line) {
        if line.args.is_empty() {
            // where's my arg?
            return;
        }
        match line.prefix {
            Some(ref user) => {
                if user.nick() == conn.user.nick() {
                    conn.user = conn.user.with_nick(line.args[0]);
                }
            }
            None => ()
        }
    }
}
