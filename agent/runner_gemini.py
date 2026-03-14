import json
import logging
import os

from google import genai
from google.genai import types

from .tools import build_tools

logger = logging.getLogger("agent")

# Stawki gemini-2.5-flash (03.2025)
_PRICE_INPUT_PER_1M = 0.075
_PRICE_OUTPUT_PER_1M = 0.30


def _log_cost(total_input: int, total_output: int) -> None:
    ci = total_input / 1_000_000 * _PRICE_INPUT_PER_1M
    co = total_output / 1_000_000 * _PRICE_OUTPUT_PER_1M
    logger.info(
        "─── TOKENY [gemini] ───  in: %d ($%.5f)  out: %d ($%.5f)  RAZEM: $%.5f",
        total_input, ci, total_output, co, ci + co,
    )


def run_agent(
    task: str,
    system_prompt: str = "",
    model: str = "gemini-3-flash-preview",
    corpus: str = "",
    max_turns: int = 20,
    on_event=None,
) -> str:
    if not corpus:
        corpus = os.getenv("AGENT_CORPUS", "")
    if not corpus:
        raise ValueError("corpus must be provided (or set AGENT_CORPUS env var)")

    client = genai.Client()
    tools = build_tools(corpus)

    config = types.GenerateContentConfig(
        system_instruction=system_prompt or None,
        tools=tools,
        automatic_function_calling=types.AutomaticFunctionCallingConfig(disable=True),
    )

    tool_map = {f.__name__: f for f in tools}
    contents: list = [types.Content(role="user", parts=[types.Part(text=task)])]

    total_input = 0
    total_output = 0

    for turn in range(1, max_turns + 1):
        if on_event:
            on_event("thinking", {"turn": turn})

        response = client.models.generate_content(
            model=model,
            contents=contents,
            config=config,
        )

        usage = response.usage_metadata
        if usage:
            total_input += usage.prompt_token_count or 0
            total_output += usage.candidates_token_count or 0

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
                logger.info("[%s] [Turn %d] FUNCTION_CALL %s | args: %s", model, turn, fc.name, args_str)
                if on_event:
                    on_event("tool_call", {"turn": turn, "tool": fc.name, "args": dict(fc.args)})

                fn = tool_map.get(fc.name)
                if fn is None:
                    tool_result = f"Nieznane narzędzie: {fc.name}"
                    status = "error"
                else:
                    try:
                        tool_result = fn(**dict(fc.args))
                        status = "OK"
                    except Exception as e:
                        tool_result = str(e)
                        status = "error"

                logger.info("[%s] [Turn %d] Tool result: %d chars | %s", model, turn, len(tool_result), status)
                logger.debug("[%s] [Turn %d] tool_result preview: %s", model, turn, tool_result[:200])
                if on_event:
                    on_event("tool_result", {"turn": turn, "tool": fc.name, "chars": len(tool_result), "status": status})

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
            logger.info("[%s] [Turn %d] TEXT → final answer (%d chars)", model, turn, len(answer))
            if on_event:
                on_event("answer", {"text": answer})
            _log_cost(total_input, total_output)
            return answer

    _log_cost(total_input, total_output)
    raise RuntimeError(f"max_turns ({max_turns}) exceeded")
