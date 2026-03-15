# /// script
# dependencies = ["google-genai", "openai", "python-dotenv"]
# ///


def run_agent(
    task: str,
    system_prompt: str = "",
    model: str = "gemini-3-flash-preview",
    corpus: str = "",
    max_turns: int = 20,
    on_event=None,
) -> str:
    if model.startswith(("gpt-", "o1", "o3", "o4")):
        from .runner_openai import run_agent as _run
    elif model.startswith("glm"):
        from .runner_zai import run_agent as _run
    else:
        from .runner_gemini import run_agent as _run

    return _run(
        task=task,
        system_prompt=system_prompt,
        model=model,
        corpus=corpus,
        max_turns=max_turns,
        on_event=on_event,
    )


if __name__ == "__main__":
    import logging
    import sys
    from pathlib import Path

    from dotenv import load_dotenv
    load_dotenv()

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
    for noisy in ("httpcore", "httpx", "google", "openai"):
        logging.getLogger(noisy).setLevel(logging.WARNING)

    answer = run_agent(task=task, system_prompt=system)
    print("\n─── ODPOWIEDŹ ───")
    print(answer)
