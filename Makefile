LIBNAME := $(shell rustc --crate-file-name lib.rs)

.PHONY: all lib clean test example

lib: $(LIBNAME)

all: lib example doc

.INTERMEDIATE: .lib.d.tmp
$(LIBNAME):
	rustc --dep-info lib.d -O lib.rs
	@sed -n -e 'p;s/lib.*:/doc:/p' lib.d > .lib.d.tmp
	@mv -f .lib.d.tmp lib.d

include lib.d

example: example/ircbot

example/ircbot: example/example.rs $(LIBNAME)
	rustc -L . -O $<

doc:
	rustdoc lib.rs
	@touch doc

clean:
	-rm -f ircbot
	-rm -f $(LIBNAME) test-irc

test: test-irc
	env RUST_THREADS=1 ./test-irc $(TESTNAME)

test-irc:
	rustc --dep-info test.d -O --test -o test-irc lib.rs

include test.d
