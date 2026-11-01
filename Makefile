CARGO ?= cargo
PREFIX ?= $(HOME)/.local
VERSION = 0.4.0

.PHONY: build test install dist clean
build:
	$(CARGO) build --release --locked

test:
	$(CARGO) test --locked

install: build
	install -Dm755 target/release/astrarium $(DESTDIR)$(PREFIX)/bin/astrarium

dist:
	mkdir -p dist
	tar --exclude='__pycache__' --exclude='*.py[cod]' --transform='s,^,astrarium-$(VERSION)/,' -czf dist/astrarium-$(VERSION).tar.gz Cargo.toml Cargo.lock src data scripts packaging assets docs .github .editorconfig .gitattributes .gitignore rust-toolchain.toml Makefile LICENSE README.md CONTRIBUTING.md CHANGELOG.md
	cp packaging/PKGBUILD dist/PKGBUILD
	sed -i "s/sha256sums=('SKIP').*/sha256sums=('$$(sha256sum dist/astrarium-$(VERSION).tar.gz | cut -d' ' -f1)')/" dist/PKGBUILD

clean:
	$(CARGO) clean
