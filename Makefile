.PHONY: build install uninstall reinstall enable disable status clean

PREFIX ?= $(HOME)/.local
BINDIR = $(PREFIX)/bin
SYSTEMD_USER_DIR = $(HOME)/.config/systemd/user

build:
	@touch hyprline-bar/src/main.rs hyprline-notifications/src/main.rs
	cargo build --release

install: build
	@echo "Stopping services for update..."
	-@systemctl --user stop hyprline-bar.service 2>/dev/null || true
	-@systemctl --user stop hyprline-notifications.service 2>/dev/null || true
	@echo "Installing binaries to $(BINDIR)..."
	@mkdir -p $(BINDIR)
	@cp target/release/hyprline-bar $(BINDIR)/
	@cp target/release/hyprline-notifications $(BINDIR)/
	@echo "Installing systemd user services..."
	@mkdir -p $(SYSTEMD_USER_DIR)
	@cp hyprline-bar.service $(SYSTEMD_USER_DIR)/
	@cp hyprline-notifications.service $(SYSTEMD_USER_DIR)/
	@systemctl --user daemon-reload
	@echo "Installation complete!"
	@echo "Run 'make enable' to enable and start services"

uninstall:
	@echo "Stopping services..."
	-@systemctl --user stop hyprline-bar.service 2>/dev/null || true
	-@systemctl --user stop hyprline-notifications.service 2>/dev/null || true
	@echo "Disabling services..."
	-@systemctl --user disable hyprline-bar.service 2>/dev/null || true
	-@systemctl --user disable hyprline-notifications.service 2>/dev/null || true
	@echo "Removing service files..."
	@rm -f $(SYSTEMD_USER_DIR)/hyprline-bar.service
	@rm -f $(SYSTEMD_USER_DIR)/hyprline-notifications.service
	@systemctl --user daemon-reload
	@echo "Removing binaries..."
	@rm -f $(BINDIR)/hyprline-bar
	@rm -f $(BINDIR)/hyprline-notifications
	@echo "Uninstallation complete!"

reinstall: uninstall install enable
	@echo "Reinstallation complete!"

enable:
	@echo "Enabling and starting services..."
	@systemctl --user enable --now hyprline-bar.service
	@systemctl --user enable --now hyprline-notifications.service
	@echo "Services enabled and started!"

disable:
	@echo "Stopping and disabling services..."
	@systemctl --user disable --now hyprline-bar.service
	@systemctl --user disable --now hyprline-notifications.service
	@echo "Services stopped and disabled!"

status:
	@echo "=== Hyprline Bar ==="
	@systemctl --user status hyprline-bar.service --no-pager || true
	@echo ""
	@echo "=== Hyprline Notifications ==="
	@systemctl --user status hyprline-notifications.service --no-pager || true

restart:
	@echo "Restarting services..."
	@systemctl --user restart hyprline-bar.service
	@systemctl --user restart hyprline-notifications.service
	@echo "Services restarted!"

logs:
	@journalctl --user -u hyprline-bar.service -u hyprline-notifications.service -f

clean:
	cargo clean
