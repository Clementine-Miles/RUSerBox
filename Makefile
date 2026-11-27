SHELL := /bin/bash

.DEFAULT_GOAL := help

FLUTTER ?= flutter
DART ?= dart
CARGO ?= cargo
DEVICE ?=
TEST ?=

.PHONY: help deps pub-get run run-device analyze test test-one coverage \
	gen gen-build gen-build-clean gen-l10n gen-proto build build-android clean test-rust

help:
	@printf '%s\n' \
		'Usage: make <target> [VAR=value]' \
		'' \
		'Common targets:' \
		'  deps               Install Dart/Flutter dependencies' \
		'  run                Run app on default device' \
		'  run-device         Run app on a specific device: make run-device DEVICE=<id>' \
		'  analyze            Run static analysis for lib/, test/ and integration_test/' \
		'  test               Run all tests' \
		'  test-one           Run a single test: make test-one TEST=test/unit/server/disk_test.dart' \
		'  coverage           Run tests with coverage output' \
		'' \
		'Code generation:' \
		'  gen                Run build_runner and gen-l10n' \
		'  gen-build          Run build_runner build' \
		'  gen-build-clean    Clear the build cache, then run build_runner build' \
		'  gen-l10n           Regenerate localization files' \
		'  gen-proto          Regenerate the tombstone protobuf reader (needs protoc)' \
		'' \
		'Build targets:' \
		'  build, build-android  Build and verify the arm64-v8a release APK' \
		'  clean              Run flutter clean' \
		'  test-rust          cargo test, fmt check and clippy for the Rust workspace'

deps pub-get:
	$(FLUTTER) pub get

run:
	$(FLUTTER) run

run-device:
	@if [ -z "$(DEVICE)" ]; then \
		echo 'DEVICE is required. Example: make run-device DEVICE=<id>'; \
		exit 1; \
	fi
	$(FLUTTER) run -d $(DEVICE)

analyze:
	$(FLUTTER) analyze lib test integration_test

test:
	$(CARGO) build -p sbm_ffi
	$(FLUTTER) test

test-one:
	@if [ -z "$(TEST)" ]; then \
		echo 'TEST is required. Example: make test-one TEST=test/unit/server/disk_test.dart'; \
		exit 1; \
	fi
	$(CARGO) build -p sbm_ffi
	$(FLUTTER) test $(TEST)

coverage:
	$(CARGO) build -p sbm_ffi
	$(FLUTTER) test --coverage

gen: gen-build gen-l10n

gen-build:
	$(DART) run build_runner build

# For a build cache that no longer matches the tree — after a merge that adds
# generated sources, most of all. A stale `.dart_tool/build/asset_graph.json`
# does not report anything: the build stops mid-phase at 0% CPU and waits
# forever, because the failure it hits is swallowed by the step awaiting it.
gen-build-clean:
	$(DART) run build_runner clean
	$(DART) run build_runner build

gen-l10n:
	$(FLUTTER) gen-l10n

# Not part of `gen`: the schema is a vendored file that changes when a new
# Android release changes it, which is on the order of once a year, and this
# needs protoc and protoc_plugin installed. Run it after updating
# third_party/proto.
gen-proto:
	@command -v protoc >/dev/null || { echo 'protoc is not installed'; exit 1; }
	$(DART) pub global activate protoc_plugin
	# `protoc` looks for its Dart backend as a `protoc-gen-dart` executable on
	# PATH, and `pub global activate` installs that into the pub cache's `bin`
	# — which is not on PATH unless the user has put it there. Without this the
	# line above succeeds and this one fails with `protoc-gen-dart: program not
	# found`, naming a plugin nothing asked for by that name.
	PATH="$${PUB_CACHE:-$$HOME/.pub-cache}/bin:$$PATH" \
		protoc --dart_out=lib/src/proto -I third_party/proto third_party/proto/tombstone.proto

build build-android:
	scripts/release/build-android.sh

clean:
	$(FLUTTER) clean

test-rust:
	$(CARGO) test --workspace
	$(CARGO) fmt --all --check
	$(CARGO) clippy --workspace --all-targets -- -D warnings
