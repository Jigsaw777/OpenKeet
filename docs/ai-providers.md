# Choose an answer model

OpenKeet uses two kinds of models. A **speech model** turns audio into text on your PC. An **answer model** can suggest responses to typed or detected questions. Connect an answer model to try a typed question, or choose **Transcription only** to work with audio without an answer provider.

| Route | What you need | Where answer text goes |
| --- | --- | --- |
| **Transcription only** | A supported speech model for live audio | No answer request |
| **Local Ollama** | Ollama running on your PC with a downloaded local model | Ollama on your PC |
| **Claude Console** | An API key from your own Console workspace | Anthropic's API, billed to that workspace |
| **ChatGPT preview** | An eligible account, browser authorization, and plan allowance or available credits | OpenAI, under the authorization you grant |

## Local answers with Ollama

1. Install and start [Ollama for Windows](https://docs.ollama.com/windows). Download a compatible local model with Ollama.
2. Open **Settings**, then **Answers**. Choose **Ollama**, select an installed local model, and use the **Use** button for Ollama. If the list is empty, select **Refresh models**.
3. Return to **Session** and try a short typed question.

OpenKeet connects to Ollama on `127.0.0.1:11434` and filters cloud model entries. Model downloads can take several gigabytes. Memory and response time depend on model size and hardware. Start with a small model. GPU acceleration depends on the runtime and hardware. After setup, local models and speech recognition can work offline.

## Claude Console with your own API key

1. Open **Settings**, then **Answers**, and choose **Claude Console**. Select **Open Claude Console** to sign in to your workspace and create an API key.
2. Enter the key in OpenKeet and select **Check and save key**. Choose a model and use the **Use** button after reviewing the data notice.

Claude API requests consume your Console workspace's API allowance or billing. A consumer Claude subscription does not grant API access. Keep keys out of issues, screenshots, and logs. [Anthropic API authentication](https://platform.claude.com/docs/en/manage-claude/authentication)

## Sign in with ChatGPT preview

1. In **Settings** under **Answers**, choose **ChatGPT**, then **Continue with ChatGPT**.
2. Review the permissions in your browser. If the connection succeeds, select an available model and choose **Use ChatGPT**.
3. If you hit a usage limit, wait for it to reset or switch to another route. Check your ChatGPT usage and app permissions in ChatGPT settings.

The desktop code implements OpenAI's browser authorization for eligible plan usage in locally run apps. Account eligibility, available models, plan allowances, and credits come from OpenAI. The preview has protocol fixture coverage. Real account entitlement and live inference still need verification. An ordinary ChatGPT login does not grant general OpenAI API access. [OpenAI sign-in and plan usage](https://developers.openai.com/siwc/token-sharing-open-source/sign-in) · [Models and inference](https://developers.openai.com/siwc/token-sharing-open-source/models-and-inference)

For a session with no answer provider, choose **Transcription only**. If a provider is active, select **Use transcription only**. See [the data guide](privacy.md) for what happens when you switch or disconnect. [Implementation and test evidence](audit/PROVIDER_AUTHORIZATION.md)
