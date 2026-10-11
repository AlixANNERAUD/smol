# smolmachines-mcp

An MCP server that gives any agent its own branchable microVM. The agent
assembles the environment itself: it creates a machine from an image with your
project copied in, runs commands there, **branches before a risky step** and
keeps or discards the result, and saves checkpoints it can come back to.

Machines are real Linux VMs with their own kernel, booted by the engine bundled
in the `smolmachines` SDK. No daemon, no account. Set `SMOL_CLOUD_TOKEN` and the
same tools run on smol cloud.

## Add it to your agent

```bash
# Claude Code
claude mcp add smol -- npx -y smolmachines-mcp

# Codex CLI (~/.codex/config.toml)
[mcp_servers.smol]
command = "npx"
args = ["-y", "smolmachines-mcp"]

# OpenCode (opencode.json)
{ "mcp": { "smol": { "type": "local", "command": ["npx", "-y", "smolmachines-mcp"], "enabled": true } } }

# Gemini CLI (~/.gemini/settings.json), Cursor (.cursor/mcp.json), and most others
{ "mcpServers": { "smol": { "command": "npx", "args": ["-y", "smolmachines-mcp"] } } }
```

Requires Node 18+ and a host the engine supports: macOS on Apple Silicon, or
Linux with `/dev/kvm`. With `SMOL_CLOUD_TOKEN` set (or `SMOL_TARGET=cloud`),
no local virtualization is needed.

## Tools

| Tool | What it does |
|---|---|
| `smol_create` | Boot a machine from an image; optionally copy a host directory in as `/workspace`. Network on by default, `allowHosts` to restrict egress. |
| `smol_exec` | Run a shell command; returns exit code, stdout, stderr. |
| `smol_branch` | Fork a running machine into an independent child from the exact same state, in about a tenth of a second. The child is branchable too. |
| `smol_delete` | Delete a machine, or discard a branch. |
| `smol_checkpoint` | Save a machine's full state to a checkpoint; the machine keeps running. |
| `smol_restore` | Start a new machine from a checkpoint: a rewind. |
| `smol_read_file`, `smol_write_file` | Move text in and out of a machine. |
| `smol_export` | Copy a directory from a machine to the host, for results you decided to keep. |
| `smol_list` | List machines and their states. |

## The flow an agent follows

1. `smol_create` with `workspace: "/path/to/project"`. The project is copied
   in; the host copy is never touched by the machine.
2. `smol_exec` to build, test, run.
3. Before a risky change, `smol_branch`. Continue in the child. If it worked,
   keep working there or `smol_export` the results to the host; if not,
   `smol_delete` the child and the source is exactly as it was.
4. `smol_checkpoint` at a point worth returning to; `smol_restore` brings it
   back as a fresh, branchable machine.

Branches and checkpoints cost only what changed, at any depth, so an agent can
afford to try several approaches and keep one.

## Environment

| Variable | Effect |
|---|---|
| `SMOL_TARGET` | `local` (default) or `cloud`. |
| `SMOL_CLOUD_TOKEN` | Cloud API key; selects the cloud target when `SMOL_TARGET` is unset. |

## Develop

```bash
cd sdk/mcp
npm install
npm run test:e2e     # builds, then drives the server over stdio against the local engine
```
