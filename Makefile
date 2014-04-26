LIBNAME := $(shell rustc --crate-file-name src/lib.rs)

.PHONY: all lib clean test example help

lib: $(LIBNAME)

help: ;
	@echo "Makefile targets:"
	@echo "  help"
	@echo "  all"
	@echo "  lib"
	@echo "  example"
	@echo "  doc"
	@echo "  test"
	@echo "  clean"

all: lib example doc

$(LIBNAME):
	rustc -O src/lib.rs

include mk/lib.d

mk/lib.d:
	@trap 'rm -f .lib.d' ERR; \
		rustc --dep-info .lib.d --no-analysis src/lib.rs
	@sed -n -e 'p;s,^lib.*:,doc:,p;s,^doc:,mk/lib.d:,p' .lib.d > $@
	@rm -f .lib.d

example: ircbot

ircbot: example/example.rs $(LIBNAME)
	rustc -L . -O $<

doc:
	rustdoc src/lib.rs
	@touch doc

clean:
	-rm -f ircbot
	-rm -f $(LIBNAME) test-irc

test: test-irc
	env RUST_THREADS=1 ./test-irc $(TESTNAME)

test-irc:
	rustc -O --test -o test-irc src/lib.rs

include mk/test.d

mk/test.d:
	@trap 'rm -f .test.d' ERR; \
		rustc --dep-info .test.d --no-analysis -o test-irc --test src/lib.rs
	@sed -n -e 'p;s,^test-irc:,mk/test.d:,p' .test.d > $@
	@rm -f .test.d
