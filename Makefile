LIBNAME := $(shell rustc --crate-file-name lib.rs)

.PHONY: all clean test example
.DEFAULT: all

$(if $(DEBUG),$(eval CFG_DEBUG := --cfg debug))

all: $(LIBNAME)

$(LIBNAME):
	rustc --dep-info lib.d -O $(CFG_DEBUG) lib.rs

include lib.d

example: example/ircbot

example/ircbot: example/example.rs $(LIBNAME)
	rustc -L . -O $(CFG_DEBUG) $<

clean:
	-rm -f example/ircbot
	-rm -f $(LIBNAME) test-irc

test: test-irc
	env RUST_THREADS=1 ./test-irc $(TESTNAME)

test-irc:
	rustc --dep-info test.d -O $(CFG_DEBUG) --test -o test-irc lib.rs

include test.d
