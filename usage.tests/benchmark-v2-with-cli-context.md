# Benchmark v2: agent-search vs klasyczne narzędzia (z kontekstem CLI)

**Data:** 2026-03-13
**Vault:** Obsidian, 1450 plików Markdown
**Wersja:** agent-search 0.4.0
**Metoda:** 8 niezależnych subagentów (4 pary), każdy bez wiedzy o wynikach drugiego

## Warunki testu

- Agenty agent-search MIAŁY pełny opis CLI w kontekście (usage, przykłady, rekomendowaną strategię)
- Agenty klasyczne używały Grep, Glob, Read (bez zmian)
- Vault był już zaindeksowany przed testem
- Inne zapytania niż v1, żeby uniknąć efektu "widziałem wyniki"

## Wyniki szczegółowe

| Zapytanie | Metoda | Wywołania | Tokeny | Czas (s) | Pewność (1-10) | Pliki znalezione |
|---|---|---|---|---|---|---|
| **Q1: Backup/DR** | agent-search | 8 | 39k | 90 | 7 | 10 |
| | grep/glob/read | 23 (39 tools) | 32k | 110 | 7 | 12 |
| **Q2: CI/CD** | agent-search | 5 | 30k | 58 | 7 | 12 |
| | grep/glob/read | 18 (24 tools) | 60k | 81 | 8 | 16 |
| **Q3: Marketing/funnels** | agent-search | 5 | 32k | 70 | 8 | 25 |
| | grep/glob/read | 22 (34 tools) | 80k | 113 | 7 | 20 |
| **Q4: PostgreSQL** | agent-search | 7 | 37k | 77 | 7 | 16 |
| | grep/glob/read | 21 (34 tools) | 71k | 121 | 7 | 15 |

## Średnie

| Metryka | agent-search | klasyczne | Przewaga |
|---|---|---|---|
| **Wywołania** | 6.25 | 21.0 | **3.4x mniej** |
| **Tokeny** | 34k | 61k | **1.8x mniej** |
| **Czas** | 74s | 106s | **1.4x szybciej** |
| **Pewność** | 7.25 | 7.25 | remis |

## Porównanie v1 vs v2 (sam agent-search)

| Metryka | v1 (bez kontekstu) | v2 (z kontekstem) | Poprawa |
|---|---|---|---|
| Wywołania | 8.25 | 6.25 | -24% |
| Tokeny | 45k | 34k | -24% |
| Czas | 106s | 74s | -30% |

## Wnioski końcowe

### agent-search wygrywa w:
- **Efektywność** — 3.4x mniej kroków (6 vs 21 wywołań średnio)
- **Koszt tokenowy** — 1.8x mniej tokenów na to samo zadanie
- **Czas** — 1.4x szybciej (74s vs 106s)
- **Recall** — w Q3 i Q4 znalazł więcej lub tyle samo plików co grep

### Klasyczne narzędzia wygrywają w:
- **Precyzja keyword** — grep szuka dosłownie, zero false positives
- **Q2 (CI/CD)** — grep dał wyższą pewność (8 vs 7) i znalazł więcej plików (16 vs 12)
- **Elastyczność** — regex, szukanie w konkretnych plikach, łączenie kryteriów ad hoc

### Kluczowe wnioski:
1. **Danie agentowi opisu CLI w system prompcie** eliminuje cold-start overhead (-24% wywołań, -30% czasu)
2. agent-search jest optymalny jako **pierwszy krok eksploracji**, grep jako **fallback do precyzyjnych wyszukiwań**
3. Potencjalny next step: dodanie `--grep` (regex) do agent-search, żeby agent miał jedno narzędzie zamiast dwóch
