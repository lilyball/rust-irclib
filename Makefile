include rust-lua/common.mk
RUST_LUA := rust-lua/$(LIBNAME)

LIBNAME := $(shell rustc --crate-file-name lib.rs)

.PHONY: all clean test
.DEFAULT: all

all: $(LIBNAME)

$(LIBNAME): $(RUST_LUA)
	rustc --dep-info lib.d lib.rs

include lib.d

define REBUILD_DIR
.PHONY: $(1)
$(1):
	$(MAKE) -C $(dir $(1))
endef

$(if $(shell $(MAKE) -C $(dir $(RUST_LUA)) -q || echo no),\
     $(eval $(call REBUILD_DIR,$(RUST_LUA))))

clean:
	-rm -f $(LIBNAME) test-irc
	-$(MAKE) -C $(dir $(RUST_LUA)) clean

test: test-irc
	env RUST_THREADS=1 ./test-irc $(TESTNAME)

test-irc: $(RUST_LUA)
	rustc --dep-info test.d -O --test -o test-irc lib.rs

include test.d
