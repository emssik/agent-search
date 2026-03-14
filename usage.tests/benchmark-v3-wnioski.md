# Wnioski z benchmark v3 — agent-search jako narzędzie do obsługi ticketów

**Data:** 2026-03-13
**Kontekst:** Wyszukiwanie informacji w tekstowej bazie wiedzy (~1450 plików MD) w celu przygotowania draftów odpowiedzi na tickety.

---

## Główny wniosek

agent-search (BM25 + grep + hybrid) jest dobrym rozwiązaniem do obsługi ticketów, **pod warunkiem że LLM orkiestruje wyszukiwanie** — sam generuje zapytania, dobiera tryby i iteruje po wynikach.

## LLM jako query generator eliminuje słabość BM25

BM25 jest lexical — nie rozumie synonimów ani parafraz. Ale to nie problem, gdy zapytania generuje model:

- Ticket: "jak usunąć użytkownika?"
- LLM generuje: `search -q "dezaktywacja konta"`, `search -q "usunięcie studenta"`, `grep -p "Deactivete|delete|remove"`
- BM25 nie musi rozumieć synonimów — **LLM je zna i generuje warianty**

To sprawia, że **embedding search jest zbędny** dla tego use case. LLM kompensuje ograniczenia lexical search inteligentnym generowaniem zapytań.

## Optymalny wzorzec użycia

```
ticket
  → LLM generuje 3-5 zapytań (różne kąty, synonimy, tryby)
  → agent-search search -q "..." --mode files  (orientacja — które pliki)
  → agent-search search -q "..." --mode chunks (kontekst do odpowiedzi)
  → agent-search grep -p "..."                 (precyzyjne dopasowanie)
  → LLM czyta wyniki, opcjonalnie dopytuje kolejnym search
  → draft odpowiedzi
```

Kluczowe: nie jeden search, a **kilka wywołań z różnymi strategiami** (search vs grep vs hybrid). Multi-query (`-q "term1" -q "term2"`) jest wbudowane, ale osobne wywołania z różnymi trybami dają lepsze pokrycie.

## Co działa dobrze

- BM25 search szybko znajduje pliki po temacie (nie wymaga dokładnego keywordu)
- Hybrid mode (`search --grep`) łączy temat + konkretną frazę — idealne dla pytań typu "jak skonfigurować SSL na sites-x"
- `--mode chunks` z `--token-budget` daje gotowy kontekst do wrzucenia w prompt LLM-a
- Indeks buduje się raz, search jest natychmiastowy
- Dla bazy ~1500 plików markdown z techniczną dokumentacją — BM25 ze stemmingiem działa dobrze

## Gdzie agent-search daje przewagę nad klasycznym Grep

1. **Hybrid (semantic + regex)** — najwyraźniejszy win. Łączenie BM25 z regexem pokrywa więcej niż iteracyjny Grep po keywordach.
2. **Vault-wide discovery** — BM25 szybciej orientuje się w rozproszonych tematach niż iteracyjne szukanie po wielu keywordach.
3. **Trudniejsze pytania (trudność 6+)** — gdy informacja jest rozproszona po wielu plikach, BM25 daje lepszy starting point.

## Gdzie klasyczny Grep wystarcza

1. **Proste keyword lookup** — szukanie konkretnej frazy, nazwy, konfiguracji.
2. **Znana struktura plików** — gdy wiadomo w jakim katalogu szukać.
3. **Exact match** — regex precyzyjniej dopasowuje wzorce niż BM25.

## Dane z benchmarku potwierdzające

- Na 10 zadaniach agent-search wygrał z klasycznym Grep w 4-5 z 10 przypadków, głównie na trudniejszych (T04, T06, T07, T08, T10).
- Sonnet + agent-search: średnia kompletność 9.4/10, precyzja 9.1/10.
- Haiku + agent-search: kompletność 7.8/10 — narzędzie nie kompensuje w pełni słabszego modelu, ale pomaga na najtrudniejszych zadaniach (T10: 8 vs 6 bez narzędzia).
- Sonnet zużywa mniej tokenów mimo lepszych wyników — trafniejsze decyzje o tym co przeszukać i przeczytać.
