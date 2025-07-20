# Makefile

# Usage:
#   make bin				# build and move binary (default is debug)
#   make bin RELEASE=1		# build and move binary in release mode
#   make cook				# build dependencies
#   make cook RELEASE=yes	# build dependencies in release mode
#   make test				# build tests
#   make test RELEASE=true	# build tests in release mode

RELEASE ?= false
RELEASE_LOWER := $(shell printf '%s' $(RELEASE) | tr A-Z a-z)

IS_RELEASE := $(filter $(RELEASE_LOWER),true 1 yes)

ifdef IS_RELEASE
    RELEASE_FLAG := --release
    BUILD_TYPE := release
else
    RELEASE_FLAG :=
    BUILD_TYPE := debug
endif

.PHONY: bin cook test

bin:
	cargo build $(RELEASE_FLAG) -p toi_server
	mv /usr/app/target/$(BUILD_TYPE)/toi_server /usr/local/bin/toi_server

cook:
	cargo chef cook $(RELEASE_FLAG) --recipe-path recipe.json

test:
	cargo test --no-run $(RELEASE_FLAG)
