#[crate_id="github.com/kballard/rust-irclib#irc:0.1"];
#[crate_type="rlib"];

//! Library for communicating with IRC servers

#[feature(macro_rules, default_type_params)];
#[warn(missing_doc)];

use std::{fmt, str, vec};

pub mod conn;

/// Representation of an IRC user
#[deriving(Clone)]
pub struct User {
    priv raw: ~[u8],
    priv nicklen: uint,
    priv user: Option<(uint, uint)>,
    priv host: Option<(uint, uint)>
}

impl User {
    /// Parse a byte-vector into a User.
    /// The byte-vector should represent a string of the form
    ///
    ///     nickname[!username][@host]
    pub fn parse<V: CloneableVector<u8>>(v: V) -> User {
        let v = v.into_owned();

        let (mut bangloc, mut atloc) = (None, None);
        for (i, &b) in v.iter().enumerate() {
            if bangloc.is_none() && b == '!' as u8 {
                bangloc = Some(i);
            } else if b == '@' as u8 {
                atloc = Some(i);
                break;
            }
        }
        let nicklen = bangloc.or(atloc).unwrap_or(v.len());
        let user = bangloc.map(|i| (i+1, atloc.unwrap_or(v.len())));
        let host = atloc.map(|i| (i+1, v.len()));
        User{
            raw: v,
            nicklen: nicklen,
            user: user,
            host: host
        }
    }

    /// Construct a new User from source components
    pub fn new(nick: &[u8], user: Option<&[u8]>, host: Option<&[u8]>) -> User {
        let cap = nick.len() + user.map_or(0, |v| v.len()+1) +
                  host.map_or(0, |v| v.len()+1);
        let mut raw = vec::with_capacity(cap);
        raw.push_all(nick);
        if user.is_some() {
            raw.push('!' as u8);
            raw.push_all(user.unwrap());
        }
        if host.is_some() {
            raw.push('@' as u8);
            raw.push_all(host.unwrap());
        }
        // instead of constructing a User directly, lets re-parse our raw string.
        // This way passing a nick of "foo!bar" or "foo@bar" will behave "properly".
        User::parse(raw)
    }

    /// Returns the raw byte-vector that represents the User
    pub fn raw<'a>(&'a self) -> &'a [u8] {
        self.raw.as_slice()
    }

    /// Returns the nickname of the User
    pub fn nick<'a>(&'a self) -> &'a [u8] {
        self.raw.slice_to(self.nicklen)
    }

    /// Returns the username of the User, if any
    pub fn user<'a>(&'a self) -> Option<&'a [u8]> {
        self.user.map(|(a,b)| self.raw.slice(a, b))
    }

    /// Returns the hostname of the User, if any
    pub fn host<'a>(&'a self) -> Option<&'a [u8]> {
        self.host.map(|(a,b)| self.raw.slice(a,b))
    }

    /// Constructs a new User with the given nick and the username/hostname
    /// of the receiver.
    pub fn with_nick(&self, nick: &[u8]) -> User {
        User::new(nick, self.user(), self.host())
    }
}

impl Eq for User {
    fn eq(&self, other: &User) -> bool {
        self.raw == other.raw
    }
}

impl fmt::Show for User {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = str::from_utf8_lossy(self.raw.as_slice());
        f.pad(s.as_slice())
    }
}

#[cfg(test)]
mod tests {
    use super::User;

    macro_rules! b(
        ($args:tt) => (
            { static b: &'static [u8] = bytes!($args); b }
        )
    )

    #[test]
    fn test_user_new() {
        let user = User::new(b!("nick"), Some(b!("user")), Some(b!("host")));
        assert_eq!(user.raw(), b!("nick!user@host"));
        assert_eq!(user.nick(), b!("nick"));
        assert_eq!(user.user(), Some(b!("user")));
        assert_eq!(user.host(), Some(b!("host")));

        let user = User::new(b!("nick"), Some(b!("user")), None);
        assert_eq!(user.raw(), b!("nick!user"));
        assert_eq!(user.nick(), b!("nick"));
        assert_eq!(user.user(), Some(b!("user")));
        assert_eq!(user.host(), None);

        let user = User::new(b!("nick"), None, Some(b!("host")));
        assert_eq!(user.raw(), b!("nick@host"));
        assert_eq!(user.nick(), b!("nick"));
        assert_eq!(user.user(), None);
        assert_eq!(user.host(), Some(b!("host")));

        let user = User::new(b!("nick"), None, None);
        assert_eq!(user.raw(), b!("nick"));
        assert_eq!(user.nick(), b!("nick"));
        assert_eq!(user.user(), None);
        assert_eq!(user.host(), None);
    }

    #[test]
    fn test_user_parse() {
        let user = User::parse(b!("bob!fred@joe.com"));
        assert_eq!(user.nick(), b!("bob"));
        assert_eq!(user.user(), Some(b!("fred")));
        assert_eq!(user.host(), Some(b!("joe.com")));

        let user = User::parse(b!("frob@whatever"));
        assert_eq!(user.nick(), b!("frob"));
        assert_eq!(user.user(), None);
        assert_eq!(user.host(), Some(b!("whatever")));

        let user = User::parse(b!("foo!baz"));
        assert_eq!(user.nick(), b!("foo"));
        assert_eq!(user.user(), Some(b!("baz")));
        assert_eq!(user.host(), None);

        let user = User::parse(b!("frobnitz"));
        assert_eq!(user.nick(), b!("frobnitz"));
        assert_eq!(user.user(), None);
        assert_eq!(user.host(), None);

        let user = User::parse(b!("host.ircserver.com"));
        assert_eq!(user.nick(), b!("host.ircserver.com"));
        assert_eq!(user.user(), None);
        assert_eq!(user.host(), None);
    }
}
