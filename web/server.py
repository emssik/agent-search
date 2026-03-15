import asyncio
import json
import logging
import os
import subprocess
from pathlib import Path

from dotenv import load_dotenv
from fastapi import FastAPI, Request
from fastapi.responses import HTMLResponse, JSONResponse
from fastapi.staticfiles import StaticFiles
from sse_starlette.sse import EventSourceResponse

load_dotenv()

logger = logging.getLogger("web")
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s %(levelname)-5s %(message)s",
    datefmt="%Y-%m-%d %H:%M:%S",
)
for noisy in ("httpcore", "httpx", "google", "openai"):
    logging.getLogger(noisy).setLevel(logging.WARNING)

CORPUS = os.getenv("AGENT_CORPUS", "/corpus")
STEMMER_LANGUAGE = os.getenv("STEMMER_LANGUAGE", "pl")
MODEL_LEFT = os.getenv("AGENT_MODEL_LEFT", os.getenv("AGENT_MODEL", "gemini-3-flash-preview"))
MODEL_CENTER = os.getenv("AGENT_MODEL_CENTER", "glm-4.7-flash")
MODEL_RIGHT = os.getenv("AGENT_MODEL_RIGHT", "gpt-5-mini")

WEB_DIR = Path(__file__).parent
SYSTEM_PROMPT_FILE = os.getenv("SYSTEM_PROMPT_FILE", str(WEB_DIR / "system_prompt.md"))

app = FastAPI(title="agent-search")

app.mount("/static", StaticFiles(directory=str(WEB_DIR / "static")), name="static")


def _load_system_prompt() -> str:
    p = Path(SYSTEM_PROMPT_FILE)
    try:
        mt = p.stat().st_mtime
    except FileNotFoundError:
        logger.warning("System prompt file not found: %s", SYSTEM_PROMPT_FILE)
        return ""
    if mt != _load_system_prompt._mtime:
        _load_system_prompt._text = p.read_text().replace("{{CORPUS}}", CORPUS)
        _load_system_prompt._mtime = mt
    return _load_system_prompt._text

_load_system_prompt._mtime = 0
_load_system_prompt._text = ""

_INDEX_HTML = (WEB_DIR / "static" / "index.html").read_text()


@app.get("/", response_class=HTMLResponse)
async def index():
    return _INDEX_HTML


@app.get("/api/health")
async def health():
    corpus_path = Path(CORPUS)
    index_exists = (corpus_path / ".agent-search-index" / "manifest.json").exists()
    return {
        "status": "ok",
        "corpus": str(corpus_path),
        "index_exists": index_exists,
    }


@app.post("/api/reindex")
async def reindex():
    loop = asyncio.get_event_loop()
    result = await loop.run_in_executor(None, _do_reindex)
    return result


def _do_reindex():
    args = ["agent-search", "index", "-c", CORPUS, "--language", STEMMER_LANGUAGE]
    result = subprocess.run(args, capture_output=True, text=True)
    return {
        "success": result.returncode == 0,
        "output": result.stdout + result.stderr,
    }


@app.get("/api/config")
async def config():
    return {"model_left": MODEL_LEFT, "model_center": MODEL_CENTER, "model_right": MODEL_RIGHT}


@app.post("/api/chat")
async def chat(request: Request):
    body = await request.json()
    question = body.get("question", "").strip()
    model = body.get("model", MODEL_LEFT)
    if not question:
        return JSONResponse({"error": "empty question"}, status_code=400)

    queue: asyncio.Queue = asyncio.Queue()
    loop = asyncio.get_event_loop()

    def on_event(event_type: str, data: dict):
        loop.call_soon_threadsafe(queue.put_nowait, (event_type, data))

    async def run_in_thread():
        from agent.runner import run_agent
        system_prompt = _load_system_prompt()
        try:
            answer = await loop.run_in_executor(
                None,
                lambda: run_agent(
                    task=question,
                    system_prompt=system_prompt,
                    model=model,
                    corpus=CORPUS,
                    on_event=on_event,
                ),
            )
        except Exception as e:
            logger.exception("Agent error")
            loop.call_soon_threadsafe(
                queue.put_nowait, ("error", {"text": str(e)})
            )
        finally:
            loop.call_soon_threadsafe(queue.put_nowait, None)

    task = asyncio.create_task(run_in_thread())

    async def event_generator():
        try:
            while True:
                try:
                    item = await asyncio.wait_for(queue.get(), timeout=300)
                except asyncio.TimeoutError:
                    yield {"event": "error", "data": json.dumps({"text": "timeout"})}
                    break
                if item is None:
                    break
                event_type, data = item
                yield {"event": event_type, "data": json.dumps(data, ensure_ascii=False)}
        finally:
            if not task.done():
                task.cancel()

    return EventSourceResponse(event_generator())
