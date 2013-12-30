//! Built-in IRC message handlers

use conn::{Conn, Line};

// 001
pub fn RPL_WELCOME(conn: &mut Conn, line: &Line) {

}

// 433
pub fn ERR_NICKNAMEINUSE(conn: &mut Conn, line: &Line) {

}
