# Provider authorization audit

The Windows preview configures answer providers under **Settings**, then **Answers**. Typed and detected questions and the opt-in local bridge use the selected provider. The archived Python prototype used separate CLI workflows. [Desktop provider guide](../ai-providers.md) · [Bridge setup](../legacy-python.md#run-the-text-bridge)

| Route | Verified documentation | Integration boundary |
| --- | --- | --- |
| ChatGPT | [Registration and sign-in](https://developers.openai.com/siwc/token-sharing-open-source/sign-in), [Models and inference](https://developers.openai.com/siwc/token-sharing-open-source/models-and-inference) | Browser consent, local loopback callback with state, nonce and PKCE, and one saved account registration. The selected ChatGPT catalog supplies eligible models, including coding models when offered. Inference uses streaming Responses. |
| Claude | [Authentication and credential use](https://code.claude.com/docs/en/legal-and-compliance#authentication-and-credential-use), [Claude API authentication](https://platform.claude.com/docs/en/manage-claude/authentication) | Open Claude Console for account sign-in. Save a key from the user's own API workspace to stream the Messages API. API usage is billed to that workspace. Anthropic restricts Claude.ai sign-in in third-party apps and the use of consumer subscription credentials on users' behalf. |
| Ollama | [List models](https://docs.ollama.com/api/tags), [Chat API](https://docs.ollama.com/api/chat) | The app connects to `127.0.0.1:11434` and requires an installed local model. Local service access needs no account. The route filters remote models before generation. |

Cloud route activation requires confirmation in Providers. The backend keeps credentials in a Windows DPAPI-protected file and scopes each request to the selected route and model. The UI receives status, model choices and the account label. Requests have bounded reads, a timeout and cancellation. Changing or removing a credential disables its selected route. ChatGPT disconnect clears local tokens and attempts server-side revocation.

The ChatGPT route provides text inference from the approved plan catalog. The Codex App Server agent, a separate OpenAI API key route and additional provider accounts remain outside this build. Local HTTP fixtures exercise the three stream formats. A signed-in account and real inference are required to verify entitlement, cloud quotas and real model response quality.
