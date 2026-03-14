# Benchmark v1: agent-search vs klasyczne narzędzia (bez kontekstu CLI)

**Data:** 2026-03-13
**Vault:** Obsidian, 1450 plików Markdown
**Wersja:** agent-search 0.4.0
**Metoda:** 8 niezależnych subagentów (4 pary), każdy bez wiedzy o wynikach drugiego

## Warunki testu

- Agenty agent-search NIE miały opisu CLI w kontekście — musiały wywołać `--help` samodzielnie (2 dodatkowe kroki)
- Agenty klasyczne używały Grep, Glob, Read
- Vault był już zaindeksowany przed testem

## Wyniki szczegółowe

| Zapytanie | Metoda | Wywołania narzędzi | Tokeny zużyte | Czas (s) | Pewność (1-10) | Pliki znalezione |
|---|---|---|---|---|---|---|
| **Q1: Docker/deployment** | agent-search | 9 | 50k | 98 | 7 | 20 |
| | grep/glob/read | 22 | 87k | 105 | 7 | 16 |
| **Q2: Payments/Stripe** | agent-search | 8 | 34k | 104 | 7 | 15 |
| | grep/glob/read | 18 | 31k | 82 | 7 | 14 |
| **Q3: Mentoring/kursy** | agent-search | 8 | 41k | 109 | 8 | 30+ |
| | grep/glob/read | 20 | 74k | 108 | 8 | 18+ |
| **Q4: Nginx/SSL/proxy** | agent-search | 8 | 55k | 113 | 8 | 12 |
| | grep/glob/read | 16 | 95k | 92 | 8 | 12 |

## Średnie

| Metryka | agent-search | klasyczne | Różnica |
|---|---|---|---|
| **Wywołania narzędzi** | 8.25 | 19.0 | 2.3x mniej |
| **Tokeny** | 45k | 72k | 1.6x mniej |
| **Czas** | 106s | 99s | ~równy |
| **Pewność** | 7.5 | 7.5 | remis |

## Obserwacje

- Agenty agent-search marnowały 2 wywołania na `--help` (nie znały składni)
- Mimo to, 2.3x mniej kroków niż klasyczne
- Agenty samodzielnie wypracowały strategię drill-down: summary → files → chunks
- Agenty dynamicznie regulowały `--max-results` i `--token-budget`
- Wszystkie nowe features (--mode, multi-query, --max-results) były aktywnie używane
