import glob as glob_module
import subprocess
from pathlib import Path


def _run_agent_search(args: list[str], corpus: str) -> tuple[str, int]:
    result = subprocess.run(
        ["agent-search", *args, "--corpus", corpus],
        capture_output=True,
        text=True,
    )
    output = result.stdout + result.stderr
    return output, result.returncode


def build_tools(corpus: str) -> list:
    def search(
        query: list[str],
        mode: str = "chunks",
        max_results: int = 10,
        token_budget: int = 4000,
        context_lines: int = 10,
        grep_filter: str = "",
        include: str = "",
    ) -> str:
        args = ["search"]
        for q in query:
            args += ["-q", q]
        args += [
            "--mode", mode,
            "--max-results", str(max_results),
            "--token-budget", str(token_budget),
            "--context-lines", str(context_lines),
        ]
        if grep_filter:
            args += ["--grep", grep_filter]
        if include:
            args += ["--include", include]
        output, returncode = _run_agent_search(args, corpus)
        if returncode != 0:
            raise RuntimeError(f"agent-search search failed (exit {returncode}): {output}")
        return output

    def grep(
        pattern: str,
        mode: str = "chunks",
        max_results: int = 100,
        token_budget: int = 4000,
        context_lines: int = 2,
        include: str = "",
    ) -> str:
        args = [
            "grep",
            "-p", pattern,
            "--mode", mode,
            "--max-results", str(max_results),
            "--token-budget", str(token_budget),
            "--context-lines", str(context_lines),
        ]
        if include:
            args += ["--include", include]
        output, returncode = _run_agent_search(args, corpus)
        if returncode != 0:
            raise RuntimeError(f"agent-search grep failed (exit {returncode}): {output}")
        return output

    def read_file(path: str, start_line: int = 1, end_line: int = 0) -> str:
        """Read file contents. start_line and end_line are 1-based (first line = 1)."""
        p = Path(path)
        if not p.is_absolute():
            p = Path(corpus) / p
        corpus_root = Path(corpus).resolve()
        if not p.resolve().is_relative_to(corpus_root):
            raise PermissionError(f"Access denied: path is outside corpus ({p})")
        lines = p.read_text().splitlines(keepends=True)
        s = max(0, start_line - 1)
        if end_line > 0:
            lines = lines[s:end_line]
        elif s > 0:
            lines = lines[s:]
        return "".join(lines)

    def glob(pattern: str) -> str:
        matches = glob_module.glob(pattern, root_dir=corpus, recursive=True)
        return "\n".join(sorted(matches)) or "No files found."

    return [search, grep, read_file, glob]
