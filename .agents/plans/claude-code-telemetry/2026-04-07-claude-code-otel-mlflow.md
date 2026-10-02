# Claude Code OTel + MLFlow Observability — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Route Claude Code telemetry (tokens, costs, prompts, tool usage) through an OTel Collector into MLFlow for analysis — without touching subscription auth.

**Architecture:** Claude Code emits OTel natively via env vars (already configured). An OTel Collector receives OTLP on `:4317`, batches, and exports to MLFlow at `:5000`. If MLFlow can't receive OTLP directly, fall back to file export + Python ingestion script.

**Tech Stack:** OpenTelemetry Collector (core, binary), MLFlow 3.11.1, Python 3.12, just

---

## File Structure

| File | Responsibility |
|------|---------------|
| `~/.local/bin/otelcol` | OTel Collector binary (installed from GitHub release) |
| `~/.config/otel/collector-config.yaml` | OTel Collector pipeline config |
| `.claude/settings.local.json` | Claude Code OTel env vars (already configured) |
| `justfile` | Add `telemetry` / `telemetry-stop` targets |
| `scripts/otel-to-mlflow.py` | Fallback ingestion script (only if Path 2 needed) |
| `telemetry/` | Fallback file export directory (only if Path 2 needed) |

---

### Task 1: Install the OTel Collector

**Files:**
- Create: `~/.local/bin/otelcol`

- [ ] **Step 1: Create the install directory**

```bash
mkdir -p ~/.local/bin
```

- [ ] **Step 2: Download the OTel Collector core binary (macOS ARM64)**

```bash
cd /tmp
curl --proto '=https' --tlsv1.2 -fOL \
  https://github.com/open-telemetry/opentelemetry-collector-releases/releases/download/v0.149.0/otelcol_0.149.0_darwin_arm64.tar.gz
```

Expected: Downloads a ~80MB tarball.

- [ ] **Step 3: Extract and install to ~/.local/bin**

```bash
tar -xzf /tmp/otelcol_0.149.0_darwin_arm64.tar.gz -C /tmp
mv /tmp/otelcol ~/.local/bin/otelcol
chmod +x ~/.local/bin/otelcol
```

- [ ] **Step 4: Verify the binary runs**

```bash
~/.local/bin/otelcol --version
```

Expected: `otelcol version 0.149.0` (or similar version string).

- [ ] **Step 5: Ensure ~/.local/bin is on PATH**

Check if `~/.local/bin` is already in your PATH:

```bash
echo $PATH | tr ':' '\n' | grep -q "$HOME/.local/bin" && echo "already on PATH" || echo "NOT on PATH"
```

If not on PATH, add to `~/.zshrc`:

```bash
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc
```

- [ ] **Step 6: Clean up**

```bash
rm -f /tmp/otelcol_0.149.0_darwin_arm64.tar.gz
```

---

### Task 2: Write the OTel Collector config

**Files:**
- Create: `~/.config/otel/collector-config.yaml`

- [ ] **Step 1: Create the config directory**

```bash
mkdir -p ~/.config/otel
```

- [ ] **Step 2: Write the collector config**

Create `~/.config/otel/collector-config.yaml`:

```yaml
receivers:
  otlp:
    protocols:
      grpc:
        endpoint: "0.0.0.0:4317"
      http:
        endpoint: "0.0.0.0:4318"

processors:
  batch:
    timeout: 5s
    send_batch_size: 100

exporters:
  otlphttp:
    endpoint: "http://localhost:5000"
    tls:
      insecure: true

  debug:
    verbosity: detailed

service:
  pipelines:
    metrics:
      receivers: [otlp]
      processors: [batch]
      exporters: [otlphttp, debug]
    logs:
      receivers: [otlp]
      processors: [batch]
      exporters: [otlphttp, debug]
    traces:
      receivers: [otlp]
      processors: [batch]
      exporters: [otlphttp, debug]
```

- [ ] **Step 3: Validate the config**

```bash
otelcol validate --config ~/.config/otel/collector-config.yaml
```

Expected: No errors. If validation passes silently, that's success.

- [ ] **Step 4: Start the collector and verify it binds**

```bash
otelcol --config ~/.config/otel/collector-config.yaml &
sleep 2
curl -s http://localhost:4318/v1/traces -X POST -H "Content-Type: application/json" -d '{}' || echo "collector not responding"
```

Expected: The collector starts, binds to `:4317` (gRPC) and `:4318` (HTTP). The curl should return a response (even an error response means the port is listening).

- [ ] **Step 5: Stop the collector**

```bash
pkill -f otelcol
```

---

### Task 3: Verify Claude Code → OTel Collector data flow

**Files:**
- Existing: `.claude/settings.local.json` (already configured)

- [ ] **Step 1: Start the OTel Collector with debug output**

```bash
otelcol --config ~/.config/otel/collector-config.yaml 2>&1 | tee /tmp/otel-debug.log &
```

- [ ] **Step 2: Start a Claude Code session**

Open a new terminal and start Claude Code in this project:

```bash
cd /Users/adam/notes/job-hunting
claude
```

Send a simple prompt like "What is 2 + 2?" then exit.

- [ ] **Step 3: Check the debug output for telemetry data**

```bash
grep -c "claude_code" /tmp/otel-debug.log
```

Expected: Non-zero count. You should see `claude_code.token.usage`, `claude_code.api_request`, or similar metric/log names in the debug output.

If zero: check that `.claude/settings.local.json` has the correct env vars and that Claude Code was started from this project directory.

- [ ] **Step 4: Inspect the actual data**

```bash
grep -A 5 "token" /tmp/otel-debug.log | head -30
```

Expected: You should see token counts (input_tokens, output_tokens) in the debug exporter output.

- [ ] **Step 5: Stop the collector**

```bash
pkill -f otelcol
rm /tmp/otel-debug.log
```

---

### Task 4: Verify MLFlow OTLP ingestion (Path 1)

This task determines whether MLFlow can receive OTLP data directly. If it can, we're done. If not, we proceed to Task 5 (fallback).

**Files:**
- None (verification only)

- [ ] **Step 1: Ensure MLFlow is running**

```bash
curl -s http://localhost:5000/health || echo "MLFlow not running — start with: mlflow server --host 0.0.0.0 --port 5000"
```

- [ ] **Step 2: Start the OTel Collector (pointed at MLFlow)**

```bash
otelcol --config ~/.config/otel/collector-config.yaml 2>&1 | tee /tmp/otel-debug.log &
```

- [ ] **Step 3: Run a Claude Code session to generate telemetry**

```bash
cd /Users/adam/notes/job-hunting
claude -p "What is 2 + 2?"
```

Using `-p` (non-interactive) so it runs a single prompt and exits.

**Note:** `-p` mode uses API key auth. If you don't have `ANTHROPIC_API_KEY` set, use interactive mode instead: open `claude`, send a prompt, then exit.

- [ ] **Step 4: Check the OTel Collector debug output for export errors**

```bash
grep -i "error\|failed\|refused" /tmp/otel-debug.log
```

Expected (Path 1 success): No errors related to the `otlphttp` exporter.
Expected (Path 1 failure): Errors like `connection refused`, `404`, `unsupported`, or `failed to export`.

- [ ] **Step 5: Check MLFlow for received traces**

```bash
curl -s http://localhost:5000/api/2.0/mlflow/traces | python3 -m json.tool | head -20
```

Expected (Path 1 success): JSON with trace data.
Expected (Path 1 failure): Empty or error response.

- [ ] **Step 6: Decision gate**

- **If Path 1 succeeded:** Skip Task 5. Proceed to Task 6.
- **If Path 1 failed:** The OTel Collector's `otlphttp` exporter endpoint may need adjustment. Try these alternatives by editing `~/.config/otel/collector-config.yaml`:
  - `endpoint: "http://localhost:5000/api/2.0/mlflow/traces"`
  - `endpoint: "http://localhost:5000/api/2.0/mlflow"`
  - Restart the collector and repeat steps 3-5.
- **If all Path 1 variants failed:** Proceed to Task 5 (file export fallback).

- [ ] **Step 7: Clean up**

```bash
pkill -f otelcol
```

---

### Task 5: Fallback — File export + Python ingestion script (only if Path 1 failed)

**Skip this task entirely if Task 4 (Path 1) succeeded.**

**Files:**
- Modify: `~/.config/otel/collector-config.yaml`
- Create: `scripts/otel-to-mlflow.py`
- Create: `telemetry/` (directory)

- [ ] **Step 1: Create the telemetry output directory**

```bash
mkdir -p /Users/adam/notes/job-hunting/telemetry
```

- [ ] **Step 2: Add a file exporter to the collector config**

Edit `~/.config/otel/collector-config.yaml`. Replace the `otlphttp` exporter with a `file` exporter:

```yaml
receivers:
  otlp:
    protocols:
      grpc:
        endpoint: "0.0.0.0:4317"
      http:
        endpoint: "0.0.0.0:4318"

processors:
  batch:
    timeout: 5s
    send_batch_size: 100

exporters:
  file/metrics:
    path: /Users/adam/notes/job-hunting/telemetry/metrics.jsonl
    flush_interval: 5s

  file/logs:
    path: /Users/adam/notes/job-hunting/telemetry/logs.jsonl
    flush_interval: 5s

  file/traces:
    path: /Users/adam/notes/job-hunting/telemetry/traces.jsonl
    flush_interval: 5s

  debug:
    verbosity: detailed

service:
  pipelines:
    metrics:
      receivers: [otlp]
      processors: [batch]
      exporters: [file/metrics, debug]
    logs:
      receivers: [otlp]
      processors: [batch]
      exporters: [file/logs, debug]
    traces:
      receivers: [otlp]
      processors: [batch]
      exporters: [file/traces, debug]
```

**Note:** The `file` exporter requires the **contrib** distribution, not core. If you installed core in Task 1, you need to download the contrib binary instead:

```bash
cd /tmp
curl --proto '=https' --tlsv1.2 -fOL \
  https://github.com/open-telemetry/opentelemetry-collector-releases/releases/download/v0.149.0/otelcol-contrib_0.149.0_darwin_arm64.tar.gz
tar -xzf otelcol-contrib_0.149.0_darwin_arm64.tar.gz
mv otelcol-contrib ~/.local/bin/otelcol
chmod +x ~/.local/bin/otelcol
```

- [ ] **Step 3: Validate and test the file exporter**

```bash
otelcol validate --config ~/.config/otel/collector-config.yaml
otelcol --config ~/.config/otel/collector-config.yaml &
sleep 2
cd /Users/adam/notes/job-hunting && claude -p "What is 2 + 2?"
sleep 5
cat telemetry/metrics.jsonl | head -5
pkill -f otelcol
```

Expected: JSON lines in `telemetry/metrics.jsonl` containing `claude_code.token.usage` data.

- [ ] **Step 4: Write the ingestion script**

Create `scripts/otel-to-mlflow.py`:

```python
#!/usr/bin/env python3
"""Ingest OTel JSON line files into MLFlow experiments."""

import json
import sys
from pathlib import Path

import mlflow

TELEMETRY_DIR = Path(__file__).parent.parent / "telemetry"
EXPERIMENT_NAME = "claude-code/sessions"


def ingest_metrics(filepath: Path) -> int:
    """Read metrics.jsonl and log each batch as an MLFlow run."""
    if not filepath.exists():
        print(f"No file: {filepath}")
        return 0

    mlflow.set_experiment(EXPERIMENT_NAME)
    count = 0

    with open(filepath) as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            try:
                record = json.loads(line)
            except json.JSONDecodeError:
                continue

            metrics = extract_metrics(record)
            if not metrics:
                continue

            with mlflow.start_run():
                mlflow.log_metrics(metrics)
                count += 1

    return count


def extract_metrics(record: dict) -> dict:
    """Pull token/cost metrics from an OTel export record."""
    metrics = {}
    resource_metrics = record.get("resourceMetrics", [])
    for rm in resource_metrics:
        for sm in rm.get("scopeMetrics", []):
            for metric in sm.get("metrics", []):
                name = metric.get("name", "")
                if "token" in name or "cost" in name:
                    data_points = (
                        metric.get("sum", {}).get("dataPoints", [])
                        or metric.get("gauge", {}).get("dataPoints", [])
                    )
                    for dp in data_points:
                        value = dp.get("asInt") or dp.get("asDouble", 0)
                        # Build a readable metric name
                        attrs = {
                            a["key"]: a["value"].get("stringValue", "")
                            for a in dp.get("attributes", [])
                        }
                        suffix = "_".join(v for v in attrs.values() if v)
                        key = f"{name}.{suffix}" if suffix else name
                        metrics[key] = value
    return metrics


if __name__ == "__main__":
    filepath = Path(sys.argv[1]) if len(sys.argv) > 1 else TELEMETRY_DIR / "metrics.jsonl"
    n = ingest_metrics(filepath)
    print(f"Ingested {n} metric batches into MLFlow experiment '{EXPERIMENT_NAME}'")
```

- [ ] **Step 5: Test the ingestion script**

```bash
cd /Users/adam/notes/job-hunting
MLFLOW_TRACKING_URI=http://localhost:5000 python3 scripts/otel-to-mlflow.py
```

Expected: `Ingested N metric batches into MLFlow experiment 'claude-code/sessions'` where N > 0.

- [ ] **Step 6: Verify in MLFlow UI**

Open `http://localhost:5000` in a browser. Check for the `claude-code/sessions` experiment and verify it has runs with token/cost metrics.

- [ ] **Step 7: Add telemetry/ to .gitignore**

```bash
echo "telemetry/" >> /Users/adam/notes/job-hunting/.gitignore
```

---

### Task 6: Add just targets for convenience

**Files:**
- Modify: `justfile`

- [ ] **Step 1: Add telemetry targets to the justfile**

Add these targets at the end of the existing `justfile`:

```justfile
# Start MLFlow + OTel Collector for telemetry
telemetry:
    @echo "Starting MLFlow (:5000) + OTel Collector (:4317)..."
    mlflow server --host 0.0.0.0 --port 5000 &
    sleep 1
    otelcol --config ~/.config/otel/collector-config.yaml &
    @echo "Telemetry stack running. Claude Code will emit OTel automatically."

# Stop the telemetry stack
telemetry-stop:
    pkill -f "mlflow server" || true
    pkill -f "otelcol" || true
    @echo "Telemetry stack stopped."
```

- [ ] **Step 2: Verify the targets work**

```bash
cd /Users/adam/notes/job-hunting
just telemetry
sleep 3
curl -s http://localhost:5000/health && echo " MLFlow OK"
curl -s http://localhost:4318/v1/traces -X POST -H "Content-Type: application/json" -d '{}' && echo " OTel OK"
just telemetry-stop
```

Expected: Both health checks pass, then both processes stop cleanly.

- [ ] **Step 3: Commit**

```bash
git add justfile
git commit -m "feat: add telemetry just targets for MLFlow + OTel Collector"
```

---

### Task 7: End-to-end smoke test

**Files:**
- None (verification only)

- [ ] **Step 1: Start the full telemetry stack**

```bash
just telemetry
```

- [ ] **Step 2: Run a Claude Code session**

```bash
cd /Users/adam/notes/job-hunting
claude
```

Send 2-3 prompts that exercise different features (a question, a file read, a tool call). Then exit.

- [ ] **Step 3: Verify data in MLFlow**

Open `http://localhost:5000` in a browser.

**If Path 1 (OTLP direct):** Check the MLFlow Traces tab for new traces with Claude Code spans.

**If Path 2 (file fallback):** Run the ingestion script, then check the experiment:

```bash
MLFLOW_TRACKING_URI=http://localhost:5000 python3 scripts/otel-to-mlflow.py
```

- [ ] **Step 4: Verify you can answer the key questions**

Confirm you can find:
- Total tokens used in the session (input + output)
- Cost of the session
- Which tool calls were made
- Prompt content (if `OTEL_LOG_USER_PROMPTS=1` is set)

- [ ] **Step 5: Remove the debug exporter**

Once everything is verified, edit `~/.config/otel/collector-config.yaml`:
- Remove the `debug:` exporter block
- Remove `debug` from all pipeline exporter lists

This stops the verbose stdout logging that was only needed during setup.

- [ ] **Step 6: Stop the stack**

```bash
just telemetry-stop
```

---

## Summary

| Task | What | Skip if |
|------|------|---------|
| 1 | Install OTel Collector binary | Already installed |
| 2 | Write collector config | - |
| 3 | Verify Claude Code → Collector flow | - |
| 4 | Test MLFlow OTLP ingestion (Path 1) | - |
| 5 | Fallback: file export + ingestion script | Path 1 succeeded |
| 6 | Add just targets | - |
| 7 | End-to-end smoke test | - |
