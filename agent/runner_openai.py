import json
import logging
import time

from openai import OpenAI

from .tools import build_tools, emit_stats, log_cost, resolve_corpus

logger = logging.getLogger("agent")

# Stawki gpt-4o-mini (03.2025) — zaktualizuj dla gpt-5-mini
_PRICE_INPUT_PER_1M = 0.15
_PRICE_OUTPUT_PER_1M = 0.60

_TOOL_SCHEMAS = [
    {
        "type": "function",
        "function": {
            "name": "search",
            "description": "Full-text search over the corpus. Supports BM25 ranking with multi-query fusion.",
            "parameters": {
                "type": "object",
                "properties": {
                    "query": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "List of search queries (fused with RRF).",
                    },
                    "mode": {
                        "type": "string",
                        "enum": ["chunks", "files"],
                        "default": "chunks",
                        "description": "Return mode: 'chunks' for text fragments, 'files' for file paths.",
                    },
                    "max_results": {
                        "type": "integer",
                        "default": 10,
                        "description": "Maximum number of results.",
                    },
                    "token_budget": {
                        "type": "integer",
                        "default": 4000,
                        "description": "Token budget for returned text.",
                    },
                    "context_lines": {
                        "type": "integer",
                        "default": 10,
                        "description": "Lines of context around each match.",
                    },
                    "grep_filter": {
                        "type": "string",
                        "default": "",
                        "description": "Optional regex to post-filter results.",
                    },
                    "include": {
                        "type": "string",
                        "default": "",
                        "description": "Glob pattern to restrict searched files.",
                    },
                },
                "required": ["query"],
            },
        },
    },
    {
        "type": "function",
        "function": {
            "name": "grep",
            "description": "Regex search over the corpus files.",
            "parameters": {
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Regex pattern to search for.",
                    },
                    "mode": {
                        "type": "string",
                        "enum": ["chunks", "files"],
                        "default": "chunks",
                    },
                    "max_results": {
                        "type": "integer",
                        "default": 100,
                    },
                    "token_budget": {
                        "type": "integer",
                        "default": 4000,
                    },
                    "context_lines": {
                        "type": "integer",
                        "default": 2,
                    },
                    "include": {
                        "type": "string",
                        "default": "",
                        "description": "Glob pattern to restrict searched files.",
                    },
                },
                "required": ["pattern"],
            },
        },
    },
    {
        "type": "function",
        "function": {
            "name": "read_file",
            "description": "Read file contents. start_line and end_line are 1-based (first line = 1).",
            "parameters": {
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "File path (absolute or relative to corpus root).",
                    },
                    "start_line": {
                        "type": "integer",
                        "default": 1,
                    },
                    "end_line": {
                        "type": "integer",
                        "default": 0,
                        "description": "Last line to read (0 = until end).",
                    },
                },
                "required": ["path"],
            },
        },
    },
    {
        "type": "function",
        "function": {
            "name": "glob",
            "description": "List files matching a glob pattern within the corpus.",
            "parameters": {
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Glob pattern (supports ** for recursive).",
                    },
                },
                "required": ["pattern"],
            },
        },
    },
]


def run_agent(
    task: str,
    system_prompt: str = "",
    model: str = "gpt-5-mini",
    corpus: str = "",
    max_turns: int = 20,
    on_event=None,
) -> str:
    t0 = time.time()
    corpus = resolve_corpus(corpus)

    client = OpenAI()
    tools = build_tools(corpus)
    tool_map = {f.__name__: f for f in tools}

    messages = []
    if system_prompt:
        messages.append({"role": "system", "content": system_prompt})
    messages.append({"role": "user", "content": task})

    total_input = 0
    total_output = 0

    for turn in range(1, max_turns + 1):
        if on_event:
            on_event("thinking", {"turn": turn})

        response = client.chat.completions.create(
            model=model,
            messages=messages,
            tools=_TOOL_SCHEMAS,
        )

        usage = response.usage
        if usage:
            total_input += usage.prompt_tokens or 0
            total_output += usage.completion_tokens or 0

        choice = response.choices[0]
        message = choice.message

        if message.tool_calls:
            messages.append(message)

            for tc in message.tool_calls:
                fn_name = tc.function.name
                try:
                    fn_args = json.loads(tc.function.arguments)
                except json.JSONDecodeError:
                    fn_args = {}

                logger.info("[%s] [Turn %d] FUNCTION_CALL %s | args: %s", model, turn, fn_name, tc.function.arguments)
                if on_event:
                    on_event("tool_call", {"turn": turn, "tool": fn_name, "args": fn_args})

                fn = tool_map.get(fn_name)
                if fn is None:
                    tool_result = f"Nieznane narzędzie: {fn_name}"
                    status = "error"
                else:
                    try:
                        tool_result = fn(**fn_args)
                        status = "OK"
                    except Exception as e:
                        tool_result = str(e)
                        status = "error"

                logger.info("[%s] [Turn %d] Tool result: %d chars | %s", model, turn, len(tool_result), status)
                logger.debug("[%s] [Turn %d] tool_result preview: %s", model, turn, tool_result[:200])
                if on_event:
                    on_event("tool_result", {"turn": turn, "tool": fn_name, "chars": len(tool_result), "status": status})

                messages.append({
                    "role": "tool",
                    "tool_call_id": tc.id,
                    "content": tool_result,
                })
        else:
            answer = message.content or ""
            if not answer:
                raise RuntimeError(f"Model returned no usable text at turn {turn}")
            logger.info("[%s] [Turn %d] TEXT → final answer (%d chars)", model, turn, len(answer))
            if on_event:
                on_event("answer", {"text": answer})
            log_cost("openai", total_input, total_output, _PRICE_INPUT_PER_1M, _PRICE_OUTPUT_PER_1M)
            emit_stats(on_event, time.time() - t0, total_input, total_output, _PRICE_INPUT_PER_1M, _PRICE_OUTPUT_PER_1M)
            return answer

    log_cost("openai", total_input, total_output, _PRICE_INPUT_PER_1M, _PRICE_OUTPUT_PER_1M)
    raise RuntimeError(f"max_turns ({max_turns}) exceeded")
