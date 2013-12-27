include rust-lua/common.mk
RUST_LUA := rust-lua/$(LIBNAME)

LIBNAME := $(shell rustc --crate-file-name lib.rs)

.PHONY: all clean
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
	-rm -f $(LIBNAME)
	-$(MAKE) -C $(dir $(RUST_LUA)) clean
