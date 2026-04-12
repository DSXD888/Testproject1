# Export Current Session Conversation Log

Export the full conversation history from this Claude Code session to a readable markdown file. The `/export` command only captures what's visible in the terminal — I need the COMPLETE conversation from the JSONL file.

## Steps

1. Find the current session's JSONL file (the most recently modified `.jsonl` in `~/.claude/projects/` for this project directory)
2. Parse it to extract all `user` and `assistant` message entries
3. Format as a markdown document with numbered turns, timestamps, and full message content
4. Write to `docs/session-log-YYYY-MM-DD-<short-description>.md`

## Script

Run this Python script, adjusting the `session_file` path and `output_file` name:

```bash
python3 << 'PYEOF'
import json, os, glob
from datetime import datetime

# Auto-detect: find the most recently modified JSONL for this project
project_dir = os.path.expanduser("~/.claude/projects/")
cwd = os.getcwd().replace("/", "-")
project_sessions = os.path.join(project_dir, cwd)
jsonl_files = sorted(glob.glob(os.path.join(project_sessions, "*.jsonl")), key=os.path.getmtime, reverse=True)

if not jsonl_files:
    print("No session files found"); exit(1)

session_file = jsonl_files[0]
session_id = os.path.basename(session_file).replace(".jsonl", "")
print(f"Using session: {session_id} ({os.path.getsize(session_file) / 1024:.0f} KB)")

output_file = f"docs/session-log-{datetime.now().strftime('%Y-%m-%d')}.md"
os.makedirs("docs", exist_ok=True)

entries = []
with open(session_file, 'r') as f:
    for line in f:
        if line.strip():
            try: entries.append(json.loads(line))
            except: pass

conversation = []
for e in entries:
    msg_type = e.get('type')
    if msg_type == 'user':
        message = e.get('message', {})
        content = message.get('content', '') if isinstance(message, dict) else str(message)
        if isinstance(content, list):
            content = '\n'.join(p.get('text', '[non-text]') if isinstance(p, dict) else str(p) for p in content)
        if content.strip():
            conversation.append({'role': 'user', 'content': content.strip(), 'ts': e.get('timestamp', '')})
    elif msg_type == 'assistant':
        message = e.get('message', {})
        if isinstance(message, dict):
            parts = [b.get('text', '') for b in message.get('content', []) if isinstance(b, dict) and b.get('type') == 'text']
            content = '\n'.join(parts)
        else:
            content = str(message)
        if content.strip():
            conversation.append({'role': 'assistant', 'content': content.strip(), 'ts': e.get('timestamp', '')})

with open(output_file, 'w') as f:
    f.write(f"# Session Log\n\n**Session:** {session_id}\n**Exported:** {datetime.now().isoformat()}\n\n---\n\n")
    turn = 0
    for entry in conversation:
        ts = ''
        if entry['ts']:
            try:
                if isinstance(entry['ts'], str): ts = f" ({entry['ts'][:19]})"
                else: ts = f" ({datetime.fromtimestamp(entry['ts']/1000 if entry['ts']>1e12 else entry['ts']).strftime('%Y-%m-%d %H:%M')})"
            except: pass
        if entry['role'] == 'user':
            turn += 1
            f.write(f"## Turn {turn}\n\n### User{ts}\n\n")
        else:
            f.write(f"### Assistant{ts}\n\n")
        c = entry['content']
        if len(c) > 15000:
            f.write(c[:15000] + f"\n\n*[... truncated, {len(c)} chars total]*\n\n")
        else:
            f.write(c)
        f.write("\n\n---\n\n")

print(f"Written: {output_file} ({os.path.getsize(output_file)/1024:.0f} KB, {turn} turns)")
PYEOF
```

That's it. The script auto-detects the current session and writes the full conversation log.
