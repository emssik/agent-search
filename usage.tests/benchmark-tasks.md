# Benchmark tasks — agent-search vs klasyczne narzędzia

**Corpus:** `/Users/daniel/00_work/obsidian.working` (~1450 plików Markdown)
**Cel:** Porównywalny zestaw 9 zadań o rosnącej trudności. Każde zadanie wykonuje się dwukrotnie — raz z agent-search, raz z grep/glob/read — i mierzy wyniki.

## Protokół wykonania

### Setup
```bash
agent-search index -c /Users/daniel/00_work/obsidian.working
```

### Przebiegi

Każde zadanie testuje **2 podejścia** = **2 agenty równolegle**:

| Agent | Model | Podejście |
|-------|-------|-----------|
| **A-Sonnet** | Sonnet | agent-search |
| **B-Sonnet** | Sonnet | klasyczne (Grep/Glob/Read) |

### Uruchom **wszystkie 18 agentów równolegle** (w jednym message z 18 Agent tool calls):

Wszystkie agenty (9 zadań × 2 podejścia) startują jednocześnie, pracują niezależnie, nie widzą swoich wyników. Dzięki temu cały benchmark wykonuje się w czasie najwolniejszego agenta, a nie sumie wszystkich.

**Agent A (agent-search):** ma dostęp do `agent-search` (search, grep, hybrid) jako **główne narzędzie odkrywania**. Może też używać Grep/Glob/Read do dopracowania wyników (np. czytanie znalezionych plików, weryfikacja kontekstu). W prompcie subagenta **musi** być na początku instrukcja:
```
Przeczytaj najpierw /Users/daniel/00_work/projects/active/agent.search/usage.tests/context.md — to kontekst operacyjny z dokumentacją narzędzia agent-search, którego będziesz używać.
```
Dzięki temu agent nie marnuje kroków na `--help` i od razu zna optymalne wzorce użycia.

**Agent B (klasyczny):** ma dostęp do Grep, Glob, Read. Nie ma agent-search.

### Co mierzyć

| Metryka | Jak |
|---------|-----|
| **Wywołania narzędzi** | Liczba tool calls (bez wliczania końcowej odpowiedzi) |
| **Tokeny** | Suma tokenów wejście+wyjście (z panelu usage) |
| **Czas** | Od startu do końcowej odpowiedzi (sekundy) |
| **Pliki znalezione** | Ile unikalnych plików agent powołał się w odpowiedzi |
| **Kompletność (1-10)** | Czy odpowiedź pokrywa temat wyczerpująco |
| **Precyzja (1-10)** | Ile z podanych informacji jest trafnych (brak halucynacji) |

### Zasady
- Agent nie zna zadania z góry — dostaje tylko prompt
- Agent nie widzi wyników drugiego agenta
- Vault jest zaindeksowany przed startem (agent-search nie liczy `index` jako krok)
- Ostateczne wyniki zapisać jako `/Users/daniel/00_work/projects/active/agent.search/usage.tests/benchmark-v4.md`

---

## Zadania

### T01 — Exact match (easy)

> Znajdź wszystkie pliki zawierające frazę `pg_dump`. Wypisz ścieżki i kontekst użycia (backup? migracja? dokumentacja?).

**Typ:** keyword grep
**Trudność:** 2/10
**Co testuje:** literal match, grep powinien tu błyszczeć

---

### T02 — Semantic single-topic (easy)

> Jak wygląda proces obsługi nowego studenta po płatności? Opisz kroki od momentu wpłaty do aktywacji konta.

**Typ:** semantic search, temat rozproszony po wielu plikach (SalesCRM, payments, student)
**Trudność:** 3/10
**Co testuje:** BM25 powinien zebrać pliki z różnych miejsc po sensie, nie po jednym keywordzie

---

### T03 — Regex pattern (medium)

> Znajdź wszystkie pliki zawierające wzorce cron (`* * * * *` lub warianty) ORAZ systemd timery. Pogrupuj wyniki: osobno crony, osobno systemd.

**Typ:** regex + kategoryzacja
**Trudność:** 4/10
**Co testuje:** regex precyzja (cron pattern) + post-processing wyników

---

### T04 — Cross-topic discovery (medium)

> Jakie technologie/narzędzia są używane do deploymentu aplikacji? Wymień narzędzia (Docker, nginx, rsync, certbot itp.) i przy każdym podaj w jakim kontekście jest używane i w których plikach.

**Typ:** multi-keyword, rozproszony temat
**Trudność:** 5/10
**Co testuje:** eksploracja szerokiego tematu, agent musi iterować po różnych narzędziach

---

### T05 — Hybrid: semantic + regex (medium-hard)

> Znajdź wszystkie notatki o lekcjach programowania, które zawierają fragmenty kodu SQL (SELECT, INSERT, CREATE TABLE itp.). Podaj ścieżki i krótki opis czego dotyczy SQL w każdej lekcji.

**Typ:** hybrid — BM25 "lekcja programowanie" + regex na SQL keywords
**Trudność:** 6/10
**Co testuje:** hybrid mode (`search --grep`), łączenie semantyki z pattern matching

---

### T06 — Temporal + scattered (hard)

> Odtwórz chronologiczny przebieg procesu email-owego ze studentem: od powiadomienia o zbliżającej się płatności, przez brak płatności, po dezaktywację. Podaj kolejność maili i ich treść.

**Typ:** scattered across many files, wymaga złożenia sekwencji
**Trudność:** 7/10
**Co testuje:** zebranie informacji z wielu plików i złożenie w logiczną całość

---

### T07 — Infrastructure audit (hard)

> Zrób audyt infrastruktury: jakie hosty istnieją, jakie kontenery na nich działają, jakie porty/domeny są skonfigurowane, jakie mechanizmy backup/SSL są ustawione. Przedstaw jako tabelę.

**Typ:** deep multi-file analysis, wymaga korelacji danych z wielu notatek
**Trudność:** 8/10
**Co testuje:** zbieranie rozproszonych faktów i synteza w ustrukturyzowaną odpowiedź

---

### T08 — Comparative analysis (very hard)

> Porównaj podejście do nauczania w ścieżce Java/Spring vs Python/Django: struktura lekcji, narzędzia, tematy deploymentu, praca z bazą danych. Co jest wspólne, co różne?

**Typ:** comparative, wymaga przeczytania i porównania dwóch dużych zbiorów lekcji
**Trudność:** 9/10
**Co testuje:** systematyczne przeszukanie dwóch podzbiorów + analiza porównawcza

---

### T09 — Full vault synthesis (extreme)

> Jakie produkty edukacyjne (kursy, wyzwania, ścieżki, webinary) istnieją lub były planowane? Dla każdego podaj: nazwę, status (aktywny/archiwum/planowany), grupę docelową, kanał sprzedaży i lejek marketingowy jeśli jest opisany.

**Typ:** vault-wide synthesis, wymaga przeszukania projektów, archiwum, marketingu, lejków
**Trudność:** 10/10
**Co testuje:** kompleksowa eksploracja całego vault, korelacja wielu źródeł, synteza

---

## Szablon wyników

```markdown
## Wyniki: T0X — [nazwa]

| Metryka | agent-search | klasyczne |
|---------|-------------|-----------|
| Wywołania narzędzi | | |
| Tokeny (k) | | |
| Czas (s) | | |
| Pliki znalezione | | |
| Kompletność (1-10) | | |
| Precyzja (1-10) | | |

### Obserwacje
- agent-search vs klasyczne: ...
```

## Podsumowanie zbiorcze (szablon)

| Zadanie | Trudność | Wywołania A/K | Tokeny A/K | Czas A/K | Kompletność A/K | Precyzja A/K | Zwycięzca |
|---------|----------|---------------|------------|----------|-----------------|-------------|-----------|
| T01 | 2 | / | / | / | / | / | |
| T02 | 3 | / | / | / | / | / | |
| T03 | 4 | / | / | / | / | / | |
| T04 | 5 | / | / | / | / | / | |
| T05 | 6 | / | / | / | / | / | |
| T06 | 7 | / | / | / | / | / | |
| T07 | 8 | / | / | / | / | / | |
| T08 | 9 | / | / | / | / | / | |
| T09 | 10 | / | / | / | / | / | |
| **Średnia** | | / | / | / | / | / | |

### Kluczowe pytania do odpowiedzi
1. Który tryb (search/grep/hybrid) najczęściej wybierał model?
2. Przy jakim poziomie trudności agent-search zaczyna dawać przewagę?
3. Gdzie różnica w jakości jest największa — w efektywności (kroki/tokeny) czy w merytoryce?
