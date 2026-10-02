# Claude Code Observability via mitmproxy + MLFlow

**Date:** 2026-04-07
**Status:** Implemented
**Goal:** Token/cost tracking, prompt analysis, and experiment tracking for Claude Code sessions — using subscription auth (no API key).

## Constraint

Claude Code subscription (Max plan) must remain untouched. No API key required.

## Why Not Native OTel?

Claude Code documents OpenTelemetry support via `CLAUDE_CODE_ENABLE_TELEMETRY=1`, but **it does not work with subscription/OAuth auth** as of v2.1.94 (2026-04-07). Known bug — multiple GitHub issues: #13803, #16383, #16498, #15417. All confirmed-working reports used API keys or AWS Bedrock.

An OTel Collector is installed and configured at `~/.local/bin/otelcol` + `~/.config/otel/collector-config.yaml`, ready to activate if the bug is fixed.

## Architecture (Implemented)

```
┌─────────────┐    HTTPS_PROXY     ┌────────────┐         ┌──────────────┐
│ Claude Code  │──────────────────→│ mitmproxy  │────────→│  Anthropic   │
│ (CLI/agents) │                   │ (:8888)    │         │  API         │
└──────────────┘                   └─────┬──────┘         └──────────────┘
                                         │ Python addon
                                         │ extracts tokens/costs/prompts
                                         ▼
                                   ┌────────────┐
                                   │ telemetry/ │
                                   │ *.jsonl    │
                                   └─────┬──────┘
                                         │ ingestion script
                                         ▼
                                   ┌────────────┐
                                   │  MLFlow    │
                                   │ (:5000)    │
                                   └────────────┘
```

**How it works:** mitmproxy performs TLS interception on Claude Code's HTTPS traffic to `api.anthropic.com`. A Python addon parses the SSE streaming responses, extracts token usage from `message_start` and `message_delta` events, estimates cost, and writes JSONL records. A separate ingestion script pushes records into MLFlow as experiment runs.

Subscription auth is preserved — mitmproxy forwards the OAuth bearer token transparently.

## What Gets Captured

Each API request produces a JSONL record with:

| Field | Description | Example |
|-------|-------------|---------|
| `timestamp` | UTC ISO 8601 | `2026-04-08T00:36:05.387Z` |
| `message_id` | Anthropic message ID | `msg_01GFtSgFzusoMNQFeVtbG9VX` |
| `model` | Actual model used | `claude-haiku-4-5-20251001` |
| `usage.input_tokens` | Input tokens | `10` |
| `usage.output_tokens` | Output tokens | `199` |
| `usage.cache_read_input_tokens` | Cache read tokens | `0` |
| `usage.cache_creation_input_tokens` | Cache creation tokens | `79067` |
| `cost_usd` | Estimated cost (USD) | `0.0166` |
| `duration_ms` | Request duration | `4622.2` |
| `stop_reason` | Why generation stopped | `end_turn` |
| `prompt` | Last user message content (opt-in) | `Explain the Pythagorean...` |

## Components

### 1. mitmproxy (HTTPS proxy)

**Installation:** `brew install mitmproxy` (v12.2.1)

**CA certificate:** `~/.mitmproxy/mitmproxy-ca-cert.pem` (auto-generated on first run)

**Port:** 8888 (not 8080 — OrbStack uses that)

### 2. Telemetry Addon (`scripts/claude-telemetry-addon.py`)

mitmproxy Python addon that:
- Filters for `api.anthropic.com /v1/messages` requests
- Parses SSE streaming responses for `message_start` and `message_delta` events
- Extracts token usage (input, output, cache read, cache creation)
- Estimates USD cost using per-model pricing table
- Writes daily JSONL files to `telemetry/claude-YYYY-MM-DD.jsonl`
- Optionally captures prompt content (`CLAUDE_TELEMETRY_CAPTURE_PROMPTS=1`)

### 3. Ingestion Script (`scripts/ingest-telemetry.py`)

Python script that:
- Reads JSONL files from `telemetry/`
- Creates MLFlow runs in the `claude-code/sessions` experiment
- Logs metrics (tokens, cost, duration) and tags (model, message_id, stop_reason)
- Tracks ingested message IDs in `.ingested` marker files to avoid duplicates

### 4. MLFlow (tracking server)

**URL:** `http://127.0.0.1:5000` (NOT `localhost` — macOS AirPlay Receiver steals port 5000 on IPv6)

**Experiment:** `claude-code/sessions`

### 5. Claude Code Configuration

Run Claude Code through the proxy:

```bash
HTTPS_PROXY=http://localhost:8888 \
NODE_EXTRA_CA_CERTS=~/.mitmproxy/mitmproxy-ca-cert.pem \
claude
```

## Daily Workflow

```bash
# 1. Start the proxy (foreground — shows live capture)
just telemetry

# 2. Run Claude Code through the proxy
HTTPS_PROXY=http://localhost:8888 \
NODE_EXTRA_CA_CERTS=~/.mitmproxy/mitmproxy-ca-cert.pem \
claude

# 3. Ingest captured data into MLFlow
just telemetry-ingest

# 4. View in MLFlow
open http://127.0.0.1:5000
```

## What You Can Query

- "How many tokens did that agent run burn?" → filter by model/message_id in MLFlow
- "What prompts are being sent?" → enable `CLAUDE_TELEMETRY_CAPTURE_PROMPTS=1`
- "What's the cost breakdown by model?" → group MLFlow runs by model tag
- "How does my token usage trend over time?" → MLFlow experiment charts

## Known Issues

- **pypi.org, github.com reject the MITM cert** — these clients pin their own CAs. Does not affect Claude Code API traffic.
- **Cache tokens are large** (~80K cache creation tokens on trivial prompts) because Claude Code injects the full system prompt + MCP tool manifests on every request.
- **macOS AirPlay steals :5000** — always use `127.0.0.1:5000` for MLFlow, not `localhost:5000`.

## Future: OTel (when bug is fixed)

OTel Collector installed at `~/.local/bin/otelcol` (v0.149.0) with config at `~/.config/otel/collector-config.yaml`. When Claude Code fixes OTel for subscription auth, switch to the native pipeline:

```
Claude Code → OTel Collector → MLFlow
```

This eliminates the proxy and provides richer data (tool execution events, traces).

## Future: LiteLLM for Multi-Provider Routing

LiteLLM slots in when custom models (GEMMA4, GLiNER2, etc.) need a unified API gateway:

```
Claude Code → mitmproxy → Anthropic (subscription, unchanged)
Your agents/scripts → LiteLLM → custom models
                          ↓ (callbacks)
                        MLFlow
```

## Decisions Made

- **mitmproxy over LiteLLM:** LiteLLM requires API key forwarding; mitmproxy transparently passes subscription OAuth.
- **mitmproxy over native OTel:** OTel broken with subscription auth (known bug). mitmproxy works today.
- **JSONL intermediate format:** Decouples capture from ingestion. Can re-ingest, debug, or pipe elsewhere.
- **Daily JSONL files:** Simple rotation, easy to archive or delete old data.
- **`127.0.0.1` not `localhost`:** macOS AirPlay binds to `[::]:5000`, MLFlow binds to `127.0.0.1:5000`.
- **Port 8888:** Port 8080 occupied by OrbStack.
