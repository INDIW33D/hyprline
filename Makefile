.PHONY: build install uninstall reinstall enable disable status clean

PREFIX ?= $(HOME)/.local
BINDIR = $(PREFIX)/bin
SYSTEMD_USER_DIR = $(HOME)/.config/systemd/user

build:
	@rm -f target/release/hyprline-bar target/release/hyprline-notifications
	@rm -rf target/release/deps/hyprline*
	cargo build --release

install: build
	@echo "Stopping services for update..."
	-@systemctl --user stop hyprline-bar.service 2>/dev/null || true
	-@systemctl --user stop hyprline-notifications.service 2>/dev/null || true
	@echo "Killing any remaining processes..."
	-@pkill -9 -f hyprline-bar 2>/dev/null || true
	-@pkill -9 -f hyprline-notifications 2>/dev/null || true
	@sleep 2
	@echo "Verifying processes stopped..."
	@pgrep -f "hyprline-bar$$" && echo "WARNING: hyprline-bar still running!" || echo "✓ hyprline-bar stopped"
	@pgrep -f "hyprline-notifications$$" && echo "WARNING: hyprline-notifications still running!" || echo "✓ hyprline-notifications stopped"
	@echo "Installing binaries to $(BINDIR)..."
	@mkdir -p $(BINDIR)
	@rm -f $(BINDIR)/hyprline-bar $(BINDIR)/hyprline-notifications
	@sync
	@cp -f target/release/hyprline-bar $(BINDIR)/
	@cp -f target/release/hyprline-notifications $(BINDIR)/
	@sync
	@chmod +x $(BINDIR)/hyprline-bar $(BINDIR)/hyprline-notifications
	@echo "Verifying binaries copied..."
	@ls -la $(BINDIR)/hyprline-bar $(BINDIR)/hyprline-notifications
	@echo "Build timestamps:"
	@ls -la target/release/hyprline-bar target/release/hyprline-notifications
	@echo "Comparing checksums..."
	@md5sum $(BINDIR)/hyprline-bar target/release/hyprline-bar | awk '{print $$1}' | uniq -c | grep -q "2" && echo "✓ hyprline-bar checksum matches" || echo "WARNING: checksum mismatch!"
	@md5sum $(BINDIR)/hyprline-notifications target/release/hyprline-notifications | awk '{print $$1}' | uniq -c | grep -q "2" && echo "✓ hyprline-notifications checksum matches" || echo "WARNING: checksum mismatch!"
	@echo "Installing systemd user services..."
	@mkdir -p $(SYSTEMD_USER_DIR)
	@cp -f hyprline-bar.service $(SYSTEMD_USER_DIR)/
	@cp -f hyprline-notifications.service $(SYSTEMD_USER_DIR)/
	@systemctl --user daemon-reload
	@echo "Installation complete!"
	@echo "Run 'make enable' to enable and start services"

uninstall:
	@echo "Stopping services..."
	-@systemctl --user stop hyprline-bar.service 2>/dev/null || true
	-@systemctl --user stop hyprline-notifications.service 2>/dev/null || true
	@echo "Killing any remaining processes..."
	-@pkill -9 -f hyprline-bar 2>/dev/null || true
	-@pkill -9 -f hyprline-notifications 2>/dev/null || true
	@sleep 1
	@echo "Verifying processes stopped..."
	@pgrep -f hyprline-bar && echo "WARNING: hyprline-bar still running!" || echo "✓ hyprline-bar stopped"
	@pgrep -f hyprline-notifications && echo "WARNING: hyprline-notifications still running!" || echo "✓ hyprline-notifications stopped"
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
	@echo "Verifying binaries removed..."
	@test ! -f $(BINDIR)/hyprline-bar && echo "✓ hyprline-bar removed" || echo "WARNING: hyprline-bar still exists!"
	@test ! -f $(BINDIR)/hyprline-notifications && echo "✓ hyprline-notifications removed" || echo "WARNING: hyprline-notifications still exists!"
	@echo "Uninstallation complete!"

reinstall: uninstall install enable
	@echo "Reinstallation complete!"

enable:
	@echo "Enabling and starting services..."
	@systemctl --user enable hyprline-notifications.service
	@systemctl --user enable hyprline-bar.service
	@echo "Starting notifications service..."
	@systemctl --user start hyprline-notifications.service
	@echo "Waiting for notifications service to initialize..."
	@sleep 3
	@systemctl --user is-active hyprline-notifications.service || echo "Warning: notifications service not active!"
	@echo "Starting bar service..."
	@systemctl --user start hyprline-bar.service
	@sleep 5
	@systemctl --user is-active hyprline-bar.service || echo "Warning: bar service not active!"
	@echo "Verifying bars are visible..."
	@hyprctl layers | grep -q gtk4-layer-shell && echo "✓ Bars visible in Hyprland" || echo "WARNING: Bars not visible in layers!"
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
	@systemctl --user stop hyprline-bar.service 2>/dev/null || true
	@systemctl --user stop hyprline-notifications.service 2>/dev/null || true
	@sleep 1
	@systemctl --user start hyprline-notifications.service
	@echo "Waiting for notifications service..."
	@sleep 2
	@systemctl --user start hyprline-bar.service
	@echo "Services restarted!"

logs:
	@journalctl --user -u hyprline-bar.service -u hyprline-notifications.service -f

clean:
	cargo clean
