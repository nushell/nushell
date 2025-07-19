# Security Policy

As a shell and programming language Nushell provides you with great powers and the potential to do dangerous things to your computer and data. Whenever there is a risk that a malicious actor can abuse a bug or a violation of documented behavior/assumptions in Nushell to harm you this is a *security* risk.
We want to fix those issues without exposing our users to unnecessary risk. Thus we want to explain our security policy.
Additional issues may be part of *safety* where the behavior of Nushell as designed and implemented can cause unintended harm or a bug causes damage without the involvement of a third party.

## Supported Versions

As Nushell is still under very active pre-stable development, the only version the core team prioritizes for security and safety fixes is the [most recent version as published on GitHub](https://github.com/nushell/nushell/releases/latest).
Only if you provide a strong reasoning and the necessary resources, will we consider blessing a backported fix with an official patch release for a previous version.

## Reporting a Vulnerability

If you suspect that a bug or behavior of Nushell can affect security or may be potentially exploitable, please report the issue to us in private.
Either reach out to the core team on [our Discord server](https://discord.gg/NtAbbGn) to arrange a private channel or use the [GitHub vulnerability reporting form](https://github.com/nushell/nushell/security/advisories/new).
Please try to answer the following questions:
- How can we reach you for further questions?
- What is the bug? Which system of Nushell may be affected?
- Do you have proof-of-concept for a potential exploit or have you observed an exploit in the wild?
- What is your assessment of the severity based on what could be impacted should the bug be exploited?
- Are additional people aware of the issue or deserve credit for identifying the issue?

We will try to get back to you within a week with:
- acknowledging the receipt of the report
- an initial plan of how we want to address this including the primary points of contact for further communication
- our preliminary assessment of how severe we judge the issue
- a proposal for how we can coordinate responsible disclosure (e.g. how we ship the bugfix, if we need to coordinate with distribution maintainers, when you can release a blog post if you want to etc.)

For purely *safety* related issues where the impact is severe by direct user action instead of malicious input or third parties, feel free to open a regular issue. If we deem that there may be an additional *security* risk on a *safety* issue we may continue discussions in a restricted forum.
