# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2025-03-05

Initial release.

### Added

- Core component system with retained tree structure and immediate-mode rendering
- Flex-based layout engine with gap, padding, and alignment support
- Event system with keyboard, mouse, resize, focus, and paste events
- Multi-backend graphics rendering (Kitty, Sixel, Unicode blocks, Linux framebuffer)
- Terminal capability detection and geometry tracking
- Focus management with Tab/Shift-Tab navigation and tab index ordering
- Vim-style modal editing state machine (Normal, Insert, Visual, Command modes)
- Theme system with automatic color degradation for limited terminals
- i18n support with RTL text direction and locale-aware number/currency formatting
- Type-safe CSS-like styling system with selectors and priority-based cascade
- Slot system with priority layering for header/status bar content
- Headless renderer for testing components without a live terminal
- Built-in components: Container, Text, Header, StatusBar, List, TextInput,
  CommandPalette, Popup, ConfirmPopup, SplitView, ScrollableView, SlottedBar,
  Image, Animation, Logo, Title
