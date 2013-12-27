include rust-lua/common.mk
RUST_LUA := rust-lua/$(LIBNAME)

LIBNAME := $(shell rustc --crate-file-name irc.rs)

.PHONY: all clean
.DEFAULT: all

all: $(LIBNAME)

$(LIBNAME):
	rustc --dep-info irc.d irc.rs

include irc.d

clean:
	rm -f $(LIBNAME)
