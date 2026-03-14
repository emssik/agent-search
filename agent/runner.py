# /// script
# dependencies = ["google-genai", "python-dotenv"]
# ///

import glob as glob_module
import json
import logging
import os
import subprocess
from pathlib import Path

from dotenv import load_dotenv

load_dotenv()

from google import genai
from google.genai import types

logger = logging.getLogger("agent")


def _run_agent_search(args: list[str], corpus: str) -> tuple[str, int]:
    result = subprocess.run(
        ["agent-search", *args, "--corpus", corpus],
        capture_output=True,
        text=True,
    )
    output = result.stdout + result.stderr
    return output, result.returncode


def _build_tools(corpus: str) -> list:
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


# Hardkodowane stawki (gemini-2.5-flash, stan na 03.2025)
_PRICE_INPUT_PER_1M = 0.075   # USD za 1M tokenów wejściowych
_PRICE_OUTPUT_PER_1M = 0.30   # USD za 1M tokenów wyjściowych


def _log_cost_summary(total_input: int, total_output: int) -> None:
    cost_input = total_input / 1_000_000 * _PRICE_INPUT_PER_1M
    cost_output = total_output / 1_000_000 * _PRICE_OUTPUT_PER_1M
    cost_total = cost_input + cost_output
    logger.info(
        "─── TOKENY ───  input: %d ($%.5f)  output: %d ($%.5f)  RAZEM: $%.5f",
        total_input, cost_input,
        total_output, cost_output,
        cost_total,
    )


def run_agent(
    task: str,
    system_prompt: str = "",
    model: str = "gemini-3-flash-preview",
    corpus: str = "",
    max_turns: int = 20,
) -> str:
    if not corpus:
        corpus = os.getenv("AGENT_CORPUS", "")
    if not corpus:
        raise ValueError("corpus must be provided (or set AGENT_CORPUS env var)")
    client = genai.Client()
    tools = _build_tools(corpus)

    config = types.GenerateContentConfig(
        system_instruction=system_prompt or None,
        tools=tools,
        automatic_function_calling=types.AutomaticFunctionCallingConfig(disable=True),
    )

    tool_map = {f.__name__: f for f in tools}
    contents: list = [types.Content(role="user", parts=[types.Part(text=task)])]

    total_input_tokens = 0
    total_output_tokens = 0

    for turn in range(1, max_turns + 1):
        response = client.models.generate_content(
            model=model,
            contents=contents,
            config=config,
        )

        usage = response.usage_metadata
        if usage:
            total_input_tokens += usage.prompt_token_count or 0
            total_output_tokens += usage.candidates_token_count or 0

        candidate_content = response.candidates[0].content
        parts = candidate_content.parts

        function_calls = [
            p for p in parts
            if hasattr(p, "function_call") and p.function_call
        ]

        if function_calls:
            response_parts = []
            for part in function_calls:
                fc = part.function_call
                args_str = json.dumps(dict(fc.args), ensure_ascii=False)
                logger.info("[Turn %d] FUNCTION_CALL %s | args: %s", turn, fc.name, args_str)

                fn = tool_map.get(fc.name)
                if fn is None:
                    tool_result = f"Nieznane narzędzie: {fc.name}"
                    status = "error"
                else:
                    kwargs = dict(fc.args)
                    try:
                        tool_result = fn(**kwargs)
                        status = "OK"
                    except Exception as e:
                        tool_result = str(e)
                        status = "error"

                logger.info("[Turn %d] Tool result: %d chars | %s", turn, len(tool_result), status)
                logger.debug("[Turn %d] tool_result preview: %s", turn, tool_result[:200])

                response_parts.append(
                    types.Part.from_function_response(
                        name=fc.name,
                        response={"result": tool_result},
                    )
                )

            contents.append(candidate_content)
            contents.append(types.Content(role="user", parts=response_parts))
        else:
            try:
                answer = response.text
            except Exception as e:
                raise RuntimeError(f"Model returned no usable text at turn {turn}: {e}") from e
            logger.info("[Turn %d] TEXT → final answer (%d chars)", turn, len(answer))
            _log_cost_summary(total_input_tokens, total_output_tokens)
            return answer

    _log_cost_summary(total_input_tokens, total_output_tokens)
    raise RuntimeError(f"max_turns ({max_turns}) exceeded")


if __name__ == "__main__":
    import sys

    context_path = Path("usage.tests/context.md")
    try:
        system = context_path.read_text()
    except FileNotFoundError:
        raise SystemExit(f"System prompt file not found: {context_path.resolve()}")
    task = sys.argv[1] if len(sys.argv) > 1 else "Jak skonfigurowany jest backup bazy danych?"

    logging.basicConfig(
        level=logging.DEBUG,
        format="%(asctime)s %(levelname)-5s %(message)s",
        datefmt="%Y-%m-%d %H:%M:%S",
    )
    for noisy in ("httpcore", "httpx", "google"):
        logging.getLogger(noisy).setLevel(logging.WARNING)

    answer = run_agent(task=task, system_prompt=system)
    print("\n─── ODPOWIEDŹ ───")
    print(answer)
