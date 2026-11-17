# Security Policy

## Supported versions

Security fixes go into the latest release only. Update to the newest version before you report an issue.

| Platform | Supported version |
| --- | --- |
| macOS (App Store) | latest 1.x |
| iOS and iPadOS (App Store) | latest 1.x |
| Windows, Linux, Android (Rust port) | not released yet; report against `main` |

## Reporting a vulnerability

Do not open a public issue for a security problem.

Report it privately through GitHub: open the [Security tab](https://github.com/shurco/tabtool/security) and select **Report a vulnerability**. The form creates a private advisory that only the maintainers can see.

Include:

- the affected platform and app version;
- steps to reproduce, and a sample file if the issue is triggered by opening a file;
- the impact you observed or expect.

## What to expect

- An acknowledgment within 7 days.
- An assessment and a fix plan after the issue is reproduced.
- Credit in the advisory and release notes, unless you ask to stay anonymous.

## Scope

TabTool is an offline CSV editor. The App Store builds run in the App Sandbox with user-selected file access only, and have no network access, analytics, or third-party SDKs (see [docs/app-store-compliance.md](../docs/app-store-compliance.md)).

In scope: crashes, memory corruption, or file access outside the user's selection caused by a crafted CSV, TSV, or text file; data loss or corruption on save; sandbox escapes.

Out of scope: issues that need a jailbroken or already compromised device, and bugs without a security impact (use the bug report template for those).
