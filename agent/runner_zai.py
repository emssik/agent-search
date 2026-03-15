import json
import logging
import os
import time

from openai import OpenAI

from .runner_openai import _TOOL_SCHEMAS
from .tools import build_tools, emit_stats, log_cost, resolve_corpus

logger = logging.getLogger("agent")

# Stawki GLM-5 (03.2026)
# Stawki GLM-4.7-Flash (03.2026)
_PRICE_INPUT_PER_1M = 0.11
_PRICE_OUTPUT_PER_1M = 0.11

_BASE_URL = "https://api.z.ai/api/paas/v4/"


def run_agent(
    task: str,
    system_prompt: str = "",
    model: str = "glm-4.7-flash",
    corpus: str = "",
    max_turns: int = 20,
    on_event=None,
) -> str:
    t0 = time.time()
    corpus = resolve_corpus(corpus)

    client = OpenAI(
        api_key=os.environ["ZAI_API_KEY"],
        base_url=_BASE_URL,
    )
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
            log_cost("zai", total_input, total_output, _PRICE_INPUT_PER_1M, _PRICE_OUTPUT_PER_1M)
            emit_stats(on_event, time.time() - t0, total_input, total_output, _PRICE_INPUT_PER_1M, _PRICE_OUTPUT_PER_1M)
            return answer

    log_cost("zai", total_input, total_output, _PRICE_INPUT_PER_1M, _PRICE_OUTPUT_PER_1M)
    raise RuntimeError(f"max_turns ({max_turns}) exceeded")
