LIBNAME := $(shell rustc --crate-file-name lib.rs)

.PHONY: all clean test
.DEFAULT: all

all: $(LIBNAME)

$(LIBNAME):
	rustc --dep-info lib.d lib.rs

include lib.d

clean:
	-rm -f $(LIBNAME) test-irc

test: test-irc
	env RUST_THREADS=1 ./test-irc $(TESTNAME)

test-irc:
	rustc --dep-info test.d -O --test -o test-irc lib.rs

include test.d
