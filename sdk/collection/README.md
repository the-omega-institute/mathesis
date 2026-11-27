# Trusted collection delivery

Python 3.11+, no third-party dependencies. See
[Methodology papers](../../docs/METHODOLOGY_PAPERS.md) for the envelope, reproducible
packaging, immutable version rules, environment variables and tag-push template.

```sh
python3 deliver_collected_paper.py /path/to/paper-repo --dry-run
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s . -v
```

The helper supports papers, notes and methodology works. It never grants an
operator identity or bypasses server validation. Dry-run envelopes contain the
private review record; do not publish them as artifacts or CI logs.
