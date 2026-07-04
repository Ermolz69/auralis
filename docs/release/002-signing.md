# OS & Updater Signing

## Why is it needed
To absolutely confirm the authenticity of the application. 
1. **OS Code Signing** removes scary operating system warnings ("Unknown Publisher" in Windows SmartScreen, "Unidentified Developer" in macOS Gatekeeper).
2. **Tauri Updater Signing** guarantees that updates come exactly from us and have not been tampered with by hackers (protection against Man-in-the-Middle attacks).

## What does it forbid
It forbids releasing and distributing stable versions of the application without a cryptographic signature. A missing signature breaks user trust and disables the automatic update system.

## Where does it run
During the `release.yml` workflow execution, right before the generation of final installers and release artifacts.

## How to fix the error
Ensure that all certificates (Apple Developer ID / Windows EV) and Tauri private keys are correctly added to the GitHub Secrets of the repository, their validity has not expired, and passwords are up to date.

## When can an exception be made
Only during the earliest alpha/beta testing stages when certificates have not yet been physically purchased by the organization (which is why the CI steps are currently commented out as placeholders). For production releases, signing is mandatory.

## Who approves the exception
CTO or Tech Lead.
