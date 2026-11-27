SHELL := /bin/bash

.DEFAULT_GOAL := help

CARGO ?= cargo
GRADLE ?= gradle
# Mesa's software Vulkan driver, for the render test where no GPU is present.
VK_ICD ?= $(firstword $(wildcard /opt/lavapipe/lvp_icd.json /usr/share/vulkan/icd.d/lvp_icd.x86_64.json))

.PHONY: help check test test-render lint fmt-check run-desktop android-check build-android build-android-debug \
	monitor-dev monitor-deploy clean

help:
	@printf '%s\n' \
		'Usage: make <target> [VAR=value]' \
		'' \
		'  check               cargo check, the whole workspace' \
		'  test                cargo test, the whole workspace' \
		'  test-render         the Vulkan render test (lavapipe when there is no GPU)' \
		'  lint                clippy with warnings denied' \
		'  run-desktop         the app in a desktop window (development only)' \
		'  android-check       cargo check for aarch64-linux-android' \
		'  build-android       release APK, arm64-v8a (signed: see scripts/release/android-signing.sh)' \
		'  build-android-debug debug APK' \
		'  monitor-dev         the Monitor HTTP API on port 3770' \
		'  monitor-deploy      put this checkout on a test machine: make monitor-deploy HOST=<ssh host>'

check:
	$(CARGO) check --workspace --all-targets

test:
	$(CARGO) test --workspace

test-render:
	VK_ICD_FILENAMES=$(VK_ICD) $(CARGO) test -p ruserbox --test render_vulkan -- --nocapture

lint:
	$(CARGO) clippy --workspace --all-targets -- -D warnings

run-desktop:
	$(CARGO) run -p ruserbox --features desktop --bin ruserbox-desktop

android-check:
	$(CARGO) ndk -t arm64-v8a --platform 28 check -p ruserbox --lib

build-android:
	scripts/release/android-signing.sh $(GRADLE) -p android assembleRelease

build-android-debug:
	$(GRADLE) -p android assembleDebug

monitor-deploy:
	@[ -n "$(HOST)" ] || { echo 'HOST=<ssh host> is required'; exit 2; }
	@bash scripts/monitor-deploy.sh "$(HOST)"

monitor-dev:
	cd monitor && $(CARGO) run -p server_box_monitor -- serve

clean:
	$(CARGO) clean
	$(GRADLE) -p android clean
