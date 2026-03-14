# Gemini API — dokumentacja dla agenta

Źródło: ai.google.dev/gemini-api/docs (stan: marzec 2026)

## Modele

| Model ID | Opis |
|---|---|
| `gemini-3-flash-preview` | Najnowszy Flash, zalecany do nowych projektów |
| `gemini-2.5-flash` | Stabilny, najlepszy stosunek ceny do wydajności |
| `gemini-2.5-flash-lite` | Najszybszy/najtańszy |
| `gemini-flash-latest` | Alias wskazujący na aktualny Flash |

## Instalacja i setup

```bash
pip install -q -U google-genai
```

```python
# Klucz API z env var GEMINI_API_KEY
from google import genai
client = genai.Client()
```

---

## Podstawowe generowanie

```python
response = client.models.generate_content(
    model="gemini-3-flash-preview",
    contents="Twoje zapytanie"
)
print(response.text)
```

---

## Chat / rozmowa wieloturowa

```python
chat = client.chats.create(model="gemini-3-flash-preview")
response = chat.send_message("Pierwsza wiadomość")
response = chat.send_message("Kolejna wiadomość")

# Historia
for message in chat.get_history():
    print(f'{message.role}: {message.parts[0].text}')
```

Ręczna historia (styl REST):
```python
from google.genai import types

contents = [
    types.Content(role="user",  parts=[types.Part(text="Hello")]),
    types.Content(role="model", parts=[types.Part(text="Hi!")]),
    types.Content(role="user",  parts=[types.Part(text="Co dalej?")]),
]
response = client.models.generate_content(
    model="gemini-3-flash-preview",
    contents=contents,
)
```

### GenerateContentConfig — ważne parametry

```python
config = types.GenerateContentConfig(
    system_instruction="Jesteś pomocnym agentem...",
    temperature=1.0,          # domyślne dla Gemini 3
    tools=[...],
    tool_config=...,
    response_mime_type="application/json",  # dla structured output
)
```

---

## Function Calling / Tool Use

### Definicja narzędzia (dict schema)

```python
search_declaration = {
    "name": "search",
    "description": "Przeszukuje dokumenty za pomocą agent-search.",
    "parameters": {
        "type": "object",
        "properties": {
            "query": {
                "type": "string",
                "description": "Fraza do wyszukania",
            },
        },
        "required": ["query"],
    },
}
```

### Wywołanie modelu z narzędziami

```python
tools = types.Tool(function_declarations=[search_declaration])
config = types.GenerateContentConfig(tools=[tools])

contents = [
    types.Content(role="user", parts=[types.Part(text="Znajdź info o X")])
]

response = client.models.generate_content(
    model="gemini-3-flash-preview",
    contents=contents,
    config=config,
)

# Sprawdź czy model chce wywołać narzędzie
tool_call = response.candidates[0].content.parts[0].function_call
print(tool_call.name, tool_call.args)
```

### Pętla agenta — obsługa wywołania i zwrócenie wyniku

```python
# 1. Wykonaj narzędzie
result = my_tool_function(**tool_call.args)

# 2. Zbuduj odpowiedź narzędzia
function_response_part = types.Part.from_function_response(
    name=tool_call.name,
    response={"result": result},
)

# 3. Dołącz turę modelu + wynik narzędzia do historii
contents.append(response.candidates[0].content)           # turn modelu (function_call)
contents.append(types.Content(                            # wynik narzędzia
    role="user",
    parts=[function_response_part]
))

# 4. Pobierz finalną odpowiedź
final_response = client.models.generate_content(
    model="gemini-3-flash-preview",
    config=config,
    contents=contents,
)
print(final_response.text)
```

### Automatyczne wywołanie (Python SDK only)

```python
def search(query: str) -> dict:
    """Przeszukuje dokumenty.

    Args:
        query: Fraza do wyszukania.

    Returns:
        Słownik z wynikami.
    """
    # ... implementacja ...
    return {"results": [...]}

config = types.GenerateContentConfig(tools=[search])  # funkcja bezpośrednio

response = client.models.generate_content(
    model="gemini-3-flash-preview",
    contents="Znajdź coś o X",
    config=config,
)
# SDK obsługuje całą pętlę automatycznie
```

Wyłączenie automatycznego wywoływania:
```python
config = types.GenerateContentConfig(
    tools=[search],
    automatic_function_calling=types.AutomaticFunctionCallingConfig(disable=True)
)
```

### Tryby function calling

```python
tool_config = types.ToolConfig(
    function_calling_config=types.FunctionCallingConfig(
        mode="ANY",   # AUTO | ANY | NONE | VALIDATED
        allowed_function_names=["search"]
    )
)
```

### Równoległe wywołania narzędzi

```python
response = chat.send_message("Zrób kilka rzeczy naraz")

for fn in response.function_calls:
    args = ", ".join(f"{k}={v}" for k, v in fn.args.items())
    print(f"{fn.name}({args})")
```

---

## Structured Output (JSON)

```python
from pydantic import BaseModel, Field
from typing import List, Optional

class SearchResult(BaseModel):
    title: str = Field(description="Tytuł dokumentu")
    snippet: str = Field(description="Fragment pasujący do zapytania")
    score: float = Field(description="Wynik trafności 0-1")

class SearchResponse(BaseModel):
    results: List[SearchResult]
    total: int

response = client.models.generate_content(
    model="gemini-3-flash-preview",
    contents="...",
    config={
        "response_mime_type": "application/json",
        "response_json_schema": SearchResponse.model_json_schema(),
    },
)

data = SearchResponse.model_validate_json(response.text)
```

**Uwaga:** Structured output + function calling jednocześnie — tylko Gemini 3+.

---

## Schemat pełnej pętli agenta z agent-search

```python
import subprocess
import json
from google import genai
from google.genai import types

client = genai.Client()

# Definicja narzędzia agent-search
search_tool_declaration = {
    "name": "agent_search",
    "description": "Przeszukuje lokalny korpus dokumentów za pomocą agent-search CLI.",
    "parameters": {
        "type": "object",
        "properties": {
            "query": {
                "type": "string",
                "description": "Zapytanie do wyszukania",
            },
            "limit": {
                "type": "integer",
                "description": "Maksymalna liczba wyników (domyślnie 10)",
            },
        },
        "required": ["query"],
    },
}

def run_agent_search(query: str, limit: int = 10, corpus: str = "/corpus") -> dict:
    """Wywołuje agent-search CLI."""
    cmd = ["agent-search", "search", "-q", query, "--max-results", str(limit), "--corpus", corpus]
    result = subprocess.run(cmd, capture_output=True, text=True)
    return {"output": result.stdout, "error": result.stderr}

def run_agent(user_query: str, system_prompt: str = "") -> str:
    tools = types.Tool(function_declarations=[search_tool_declaration])
    config = types.GenerateContentConfig(
        system_instruction=system_prompt or "Jesteś agentem wyszukiwania. Używaj narzędzia agent_search by znaleźć odpowiedzi.",
        tools=[tools],
        tool_config=types.ToolConfig(
            function_calling_config=types.FunctionCallingConfig(mode="AUTO")
        ),
    )

    contents = [
        types.Content(role="user", parts=[types.Part(text=user_query)])
    ]

    # Pętla agenta
    while True:
        response = client.models.generate_content(
            model="gemini-3-flash-preview",
            contents=contents,
            config=config,
        )

        part = response.candidates[0].content.parts[0]

        # Sprawdź czy to wywołanie narzędzia
        if hasattr(part, "function_call") and part.function_call:
            fn = part.function_call

            if fn.name == "agent_search":
                result = run_agent_search(**fn.args)
            else:
                result = {"error": f"Nieznane narzędzie: {fn.name}"}

            function_response = types.Part.from_function_response(
                name=fn.name,
                response={"result": result},
            )

            contents.append(response.candidates[0].content)
            contents.append(types.Content(role="user", parts=[function_response]))

        else:
            # Finalna odpowiedź tekstowa
            return response.text

# Użycie
if __name__ == "__main__":
    answer = run_agent("Jak działa indeksowanie w agent-search?")
    print(answer)
```

---

## Uwagi

- Pakiet: `google-genai` (nie `google-generativeai` — to stary SDK v1)
- Klucz: env var `GEMINI_API_KEY`
- Temperatura domyślna `1.0` dla Gemini 3; `0` dla deterministycznego function calling na starszych modelach
- `client.chats.create()` — zarządza historią automatycznie
- `client.models.generate_content()` — bezstanowe, historia ręczna
- Automatyczne function calling — tylko Python SDK
