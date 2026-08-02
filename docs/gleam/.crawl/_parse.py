import re, html, sys

src = "/home/node/Development/ai-workbench/docs/gleam/.crawl/_raw-everything.html"
out = "/home/node/Development/ai-workbench/docs/gleam/.crawl/_parsed-everything.txt"
with open(src, encoding="utf-8") as f:
    doc = f.read()

lessons = re.findall(r'<article class="lesson"[^>]*id="([^"]+)"[^>]*>(.*?)</article>', doc, re.S)
print(f"lessons found: {len(lessons)}", file=sys.stderr)

def clean_text(s):
    s = re.sub(r'<code>(.*?)</code>', lambda m: '`' + html.unescape(m.group(1)) + '`', s, flags=re.S)
    s = re.sub(r'<a[^>]*>(.*?)</a>', lambda m: m.group(1), s, flags=re.S)
    s = re.sub(r'<em>(.*?)</em>', lambda m: '*' + m.group(1) + '*', s, flags=re.S)
    s = re.sub(r'<i[^>]*>(.*?)</i>', lambda m: m.group(1), s, flags=re.S)
    s = re.sub(r'<[^>]+>', '', s)
    s = html.unescape(s)
    s = re.sub(r'[ \t]+', ' ', s)
    s = re.sub(r'\n{3,}', '\n\n', s)
    return s.strip()

lines = []
for lid, body in lessons:
    tm = re.search(r'<h2[^>]*>(.*?)</h2>', body, re.S)
    title = clean_text(tm.group(1)) if tm else "(no title)"
    lines.append(f"\n\n===== LESSON: {lid} | {title} =====")
    pattern = re.compile(r'(<p[^>]*>.*?</p>)|(<pre[^>]*><code[^>]*>.*?</code>.*?</pre>)', re.S)
    for m in pattern.finditer(body):
        if m.group(1):
            txt = clean_text(m.group(1))
            if txt:
                lines.append("\n--PROSE--\n" + txt)
        else:
            cm = re.search(r'<code[^>]*>(.*?)</code>', m.group(2), re.S)
            if cm:
                code = html.unescape(cm.group(1))
                code = re.sub(r'\n$', '', code)
                lines.append("\n--CODE--\n" + code)

with open(out, "w", encoding="utf-8") as f:
    f.write("\n".join(lines))
print(f"wrote {out}", file=sys.stderr)
print(f"total blocks: {len(lines)}", file=sys.stderr)
