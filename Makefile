LIBNAME := $(shell rustc --crate-file-name lib.rs)

.PHONY: all lib clean test example help

help: ;
	@echo "Makefile targets:"
	@echo "  help"
	@echo "  all"
	@echo "  lib"
	@echo "  example"
	@echo "  doc"
	@echo "  test"
	@echo "  clean"

lib: $(LIBNAME)

all: lib example doc

.INTERMEDIATE: .lib.d.tmp
$(LIBNAME):
	rustc -O lib.rs

include lib.d

lib.d:
	rustc --dep-info .lib.d --no-analysis lib.rs
	@sed -n -e 'p;s/lib.*:/doc:/p;s/doc:/lib.d:/p' .lib.d > .lib.d~
	@mv -f .lib.d~ lib.d
	@rm -f .lib.d

example: ircbot

ircbot: example/example.rs $(LIBNAME)
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
	rustc -O --test -o test-irc lib.rs

include test.d

test.d:
	rustc --dep-info .test.d --no-analysis -o test-irc --test lib.rs
	@sed -n -e 'p;s/^test-irc:/test.d:/p' .test.d > .test.d~
	@mv -f .test.d~ test.d
	@rm -f .test.d
