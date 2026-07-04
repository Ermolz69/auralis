# 000-stack: Technology Stack

## 1. Context

This document outlines the core technology stack for the Auralis project. The primary goal of standardizing this stack is to establish clear boundaries, resolve past ambiguities (such as the choice between React and Vue), and ensure a scalable architecture that separates the UI presentation from heavy background processing.

## 2. Core Decisions

### 2.1. Frontend
- **Framework**: React + Vite + TypeScript
  - *Note: We explicitly use React. Vue is not used in this project.*
- **Styling**: Tailwind CSS
- **UI Documentation**: Storybook
- **Architecture Methodology**: Feature-Sliced Design (FSD)

### 2.2. Desktop Shell (Tauri)
- **Framework**: Tauri v2
- **Role of `src-tauri`**: The `src-tauri` directory remains a **thin shell layer**. It is strictly responsible for window management, IPC (Inter-Process Communication), and basic native OS integrations. It should not contain complex business logic or heavy processing.

### 2.3. Backend & Heavy Processing
- **Core Backend**: Rust workspace
- **Heavy Processes**: All heavy, CPU-intensive, or specialized external processes are routed through **sidecar binaries**. The main Tauri application orchestrates these sidecars rather than executing heavy tasks within the main Tauri rust process itself.

### 2.4. Tooling & DevOps
- **Task Runner**: Taskfile (for local scripts, builds, and orchestration)
- **CI/CD**: GitHub Actions
- **Documentation**: Markdown format, stored in the `docs/` directory.

## 3. Consequences
By adopting this stack:
- The frontend remains fast and strictly focused on UI concerns using a modern React ecosystem.
- Tauri stays performant as a lightweight wrapper, while heavy-lifting and complex state management are delegated to separate sidecar processes.
- FSD ensures the frontend codebase remains maintainable and decoupled as it grows.
