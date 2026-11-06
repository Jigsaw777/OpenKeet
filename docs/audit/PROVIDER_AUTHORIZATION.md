# Provider authorization audit

The Windows preview currently selects the bundled practice responder. The separate legacy Python [provider adapter](../../providers.py) invokes user-installed CLI binaries. The Rust build contains no provider login, token store, inference request, or subscription activation.

| Route | Verified documentation | Integration boundary |
| --- | --- | --- |
| OpenAI developer API | [API authentication](https://developers.openai.com/api/reference/overview#authentication) | Use separately authorized API credentials in the Rust backend. Keep credentials out of WebView state, snapshots, diagnostics and Git. |
| Sign in with ChatGPT | [Local and open-source app overview](https://developers.openai.com/siwc/token-sharing-open-source), [Codex App Server integration](https://developers.openai.com/siwc/token-sharing-open-source/codex-app-server) | Current documentation describes an app-owned host identity, per-user OAuth registration, and explicit ChatGPT plan authorization. A listed model is a catalog entry. Access requires an authorized inference turn. |
| Codex local integration | [App Server protocol](https://learn.chatgpt.com/docs/app-server), [CLI authentication](https://learn.chatgpt.com/docs/cli/reference#codex-login) | The supported protocol can own sign-in and stream events. Scope approval and command/sandbox behavior separately for a desktop client. The App Server docs describe limits on hosted and commercial usage. |
| Anthropic developer API | [Claude API authentication](https://platform.claude.com/docs/en/manage-claude/authentication) | Pin the API version, apply its documented authorization method and workspace scope, and store the credential through the OS. Verify denial and rate limiting with a user-authorized account. |
| Local models | [Legacy loopback client](../../core.py#L227) | An explicit loopback configuration is the lowest scope route. Verify connectivity, model selection, generation, cancellation and offline recovery against a real local model. |

Provider milestones require explicit provider and data-route selection, permission checks, timeouts, cancellation, and stream/revision tests. Keep direct API access and plan-authorized sign-in separate. User consent and entitlement checks remain open. The audit did not send an inference request.
