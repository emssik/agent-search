# Benchmark v3 — Haiku vs Sonnet × agent-search vs klasyczne

**Data:** 2026-03-13
**Corpus:** `/Users/daniel/00_work/obsidian.working` (~1450 plików Markdown)
**Orkiestrator:** Claude Opus 4.6
**Modele testowe:** Haiku 4.5, Sonnet 4.6

---

## Wyniki: T01 — Needle in a haystack

> Znajdź plik zawierający konfigurację hosta "puchacz". Podaj pełną ścieżkę i opisz co ten host obsługuje.

### Haiku

| Metryka | agent-search | klasyczne |
|---------|-------------|-----------|
| Wywołania narzędzi | 4 | 2 |
| Tokeny (k) | 24.5 | 22.4 |
| Czas (s) | 11.2 | 6.3 |
| Pliki znalezione | 1 | 1 |
| Kompletność (1-10) | 8 | 8 |
| Precyzja (1-10) | 10 | 10 |

### Sonnet

| Metryka | agent-search | klasyczne |
|---------|-------------|-----------|
| Wywołania narzędzi | 4 | 3 |
| Tokeny (k) | 13.1 | 10.8 |
| Czas (s) | 19.6 | 11.1 |
| Pliki znalezione | 2 | 2 |
| Kompletność (1-10) | 10 | 10 |
| Precyzja (1-10) | 10 | 10 |

### Obserwacje
- Różnica Haiku vs Sonnet: Sonnet znalazł dodatkowy plik (Puchacz API), Haiku ograniczył się do głównego pliku konfiguracji. Sonnet zużył mniej tokenów mimo lepszego wyniku.
- agent-search vs klasyczne: Na trivialnym zadaniu agent-search nie daje przewagi — Grep jest szybszy i wystarczający. Agent-search dodaje overhead (czytanie usage.md).

---

## Wyniki: T02 — Exact match (pg_dump)

> Znajdź wszystkie pliki zawierające frazę `pg_dump`. Wypisz ścieżki i kontekst użycia.

### Haiku

| Metryka | agent-search | klasyczne |
|---------|-------------|-----------|
| Wywołania narzędzi | 11 | 14 |
| Tokeny (k) | 30.3 | 59.3 |
| Czas (s) | 85.6 | 64.8 |
| Pliki znalezione | 0 | 1 (powiązany) |
| Kompletność (1-10) | 7 | 9 |
| Precyzja (1-10) | 10 | 9 |

### Sonnet

| Metryka | agent-search | klasyczne |
|---------|-------------|-----------|
| Wywołania narzędzi | 7 | 6 |
| Tokeny (k) | 12.5 | 13.8 |
| Czas (s) | 30.9 | 23.5 |
| Pliki znalezione | 0 | 0 |
| Kompletność (1-10) | 8 | 7 |
| Precyzja (1-10) | 10 | 10 |

### Obserwacje
- Różnica Haiku vs Sonnet: Haiku (oba podejścia) zużył znacznie więcej tokenów i czasu — szukał obsesyjnie wariantów. Sonnet szybko zaakceptował brak wyników.
- agent-search vs klasyczne: B-Haiku (klasyczny) znalazł powiązane wzmianki o "dumpach" — inicjatywa wyszukiwania wariantów. A-Sonnet zweryfikował działanie narzędzia kontrolnym szukaniem "postgres".
- Fakt: `pg_dump` nie istnieje w corpus.

---

## Wyniki: T03 — Semantic single-topic

> Jak wygląda proces obsługi nowego studenta po płatności? Opisz kroki od momentu wpłaty do aktywacji konta.

### Haiku

| Metryka | agent-search | klasyczne |
|---------|-------------|-----------|
| Wywołania narzędzi | 19 | 20 |
| Tokeny (k) | 62.9 | 37.5 |
| Czas (s) | 55.0 | 44.3 |
| Pliki znalezione | 5 | 8 |
| Kompletność (1-10) | 7 | 8 |
| Precyzja (1-10) | 9 | 9 |

### Sonnet

| Metryka | agent-search | klasyczne |
|---------|-------------|-----------|
| Wywołania narzędzi | 25 | 31 |
| Tokeny (k) | 41.3 | 47.9 |
| Czas (s) | 92.9 | 88.5 |
| Pliki znalezione | 7+ | 12+ |
| Kompletność (1-10) | 9 | 10 |
| Precyzja (1-10) | 9 | 9 |

### Obserwacje
- Różnica Haiku vs Sonnet: Sonnet (oba podejścia) znalazł znacznie więcej plików i włączył onboarding Discord, prolongatę. B-Sonnet najkompletniejszy (12+ plików).
- agent-search vs klasyczne: Na semantic task klasyczne narzędzia (Grep po keywordach + Glob po nazwach) okazały się porównywalne — B-Haiku znalazł 8 plików vs A-Haiku 5.

---

## Wyniki: T04 — Regex pattern (cron + systemd)

> Znajdź wszystkie pliki zawierające wzorce cron i systemd timery. Pogrupuj wyniki.

### Haiku

| Metryka | agent-search | klasyczne |
|---------|-------------|-----------|
| Wywołania narzędzi | 23 | 20 |
| Tokeny (k) | 51.8 | 52.8 |
| Czas (s) | 78.2 | 34.4 |
| Pliki znalezione | 8 | 4 |
| Kompletność (1-10) | 9 | 8 |
| Precyzja (1-10) | 8 | 9 |

### Sonnet

| Metryka | agent-search | klasyczne |
|---------|-------------|-----------|
| Wywołania narzędzi | 25 | 7 |
| Tokeny (k) | 31.3 | 11.4 |
| Czas (s) | 99.0 | 29.0 |
| Pliki znalezione | 7 | 4 |
| Kompletność (1-10) | 10 | 8 |
| Precyzja (1-10) | 9 | 10 |

### Obserwacje
- Różnica Haiku vs Sonnet: B-Sonnet był ekstremalnie efektywny (7 tools, 11k tokenów, 29s) — precyzyjny regex search. A-Sonnet najkompletniejszy (znalazł n8n cron `*/10 6-23 * * *`).
- agent-search vs klasyczne: agent-search znalazł więcej plików (kontekstowe wzmianki o cronie), ale kosztem czasu i tokenów. Klasyczny Grep wystarczył do znalezienia 4 kluczowych plików.

---

## Wyniki: T05 — Cross-topic discovery

> Jakie technologie/narzędzia są używane do deploymentu aplikacji? Wymień narzędzia i kontekst.

### Haiku

| Metryka | agent-search | klasyczne |
|---------|-------------|-----------|
| Wywołania narzędzi | 13 | 11 |
| Tokeny (k) | 92.4 | 79.6 |
| Czas (s) | 43.4 | 29.0 |
| Pliki znalezione | 7 | 6 |
| Kompletność (1-10) | 9 | 8 |
| Precyzja (1-10) | 8 | 9 |

### Sonnet

| Metryka | agent-search | klasyczne |
|---------|-------------|-----------|
| Wywołania narzędzi | 27 | 28 |
| Tokeny (k) | 82.4 | 92.8 |
| Czas (s) | 108.7 | 111.8 |
| Pliki znalezione | 7 | 13 |
| Kompletność (1-10) | 9 | 10 |
| Precyzja (1-10) | 9 | 9 |

### Obserwacje
- Różnica Haiku vs Sonnet: Sonnet znalazł więcej narzędzi (Harbor, NetBox, pm2) i powołał się na większą liczbę plików. B-Sonnet najkompletniejszy (13 plików, 16 narzędzi).
- agent-search vs klasyczne: Przy szerokim temacie oba podejścia wymagają wielu kroków. B-Sonnet (klasyczny) znalazł więcej plików niż A-Sonnet (agent-search) — iteracyjne Grep po wielu keywordach efektywniejsze.

---

## Wyniki: T06 — Hybrid: semantic + regex

> Znajdź notatki o lekcjach programowania zawierające fragmenty kodu SQL.

### Haiku

| Metryka | agent-search | klasyczne |
|---------|-------------|-----------|
| Wywołania narzędzi | 12 | 17 |
| Tokeny (k) | 73.9 | 62.7 |
| Czas (s) | 35.6 | 31.6 |
| Pliki znalezione | 9 | 12 |
| Kompletność (1-10) | 7 | 8 |
| Precyzja (1-10) | 8 | 9 |

### Sonnet

| Metryka | agent-search | klasyczne |
|---------|-------------|-----------|
| Wywołania narzędzi | 34 | 22 |
| Tokeny (k) | 55.1 | 73.0 |
| Czas (s) | 167.8 | 96.0 |
| Pliki znalezione | 22 | 13 |
| Kompletność (1-10) | 10 | 9 |
| Precyzja (1-10) | 8 | 9 |

### Obserwacje
- Różnica Haiku vs Sonnet: A-Sonnet wybitnie kompletny — znalazł 22 pliki (w tym serię Instagram SQL #10-#21). Kosztem 167s i 34 tool calls.
- agent-search vs klasyczne: To zadanie idealne dla trybu hybrid (`search --grep`). A-Sonnet wykorzystał BM25 "lekcja programowanie" + regex SQL i znalazł serie Instagram SQL, których klasyczny Grep nie pokrył.
- **Pierwszy wyraźny win dla agent-search** — na zadaniach łączących semantykę z pattern matching.

---

## Wyniki: T07 — Temporal + scattered

> Odtwórz chronologiczny przebieg procesu email-owego ze studentem.

### Haiku

| Metryka | agent-search | klasyczne |
|---------|-------------|-----------|
| Wywołania narzędzi | 24 | 20 |
| Tokeny (k) | 49.7 | 42.1 |
| Czas (s) | 53.5 | 32.7 |
| Pliki znalezione | 7 | 5 |
| Kompletność (1-10) | 8 | 7 |
| Precyzja (1-10) | 9 | 9 |

### Sonnet

| Metryka | agent-search | klasyczne |
|---------|-------------|-----------|
| Wywołania narzędzi | 14 | 13 |
| Tokeny (k) | 27.7 | 22.8 |
| Czas (s) | 83.8 | 69.7 |
| Pliki znalezione | 8 | 8 |
| Kompletność (1-10) | 10 | 9 |
| Precyzja (1-10) | 10 | 10 |

### Obserwacje
- Różnica Haiku vs Sonnet: Sonnet (oba) zużył mniej tokenów i narzędzi, a dostarczył kompletniejszą odpowiedź. A-Sonnet znalazł archiwalne maile (darmowy miesiąc, rezygnacja).
- agent-search vs klasyczne: BM25 search "proces email student płatność" szybko znalazł kluczowe pliki. Oba Sonnety miały 8 plików — agent-search dotarł do archiwalnych maili.

---

## Wyniki: T08 — Infrastructure audit

> Zrób audyt infrastruktury: hosty, kontenery, porty/domeny, backup/SSL. Tabela.

### Haiku

| Metryka | agent-search | klasyczne |
|---------|-------------|-----------|
| Wywołania narzędzi | 23 | 22 |
| Tokeny (k) | 72.0 | 91.1 |
| Czas (s) | 63.4 | 45.6 |
| Pliki znalezione | 7 | 10 |
| Kompletność (1-10) | 7 | 8 |
| Precyzja (1-10) | 8 | 9 |

### Sonnet

| Metryka | agent-search | klasyczne |
|---------|-------------|-----------|
| Wywołania narzędzi | 35 | 32 |
| Tokeny (k) | 81.7 | 88.6 |
| Czas (s) | 151.7 | 131.6 |
| Pliki znalezione | 16 | 14 |
| Kompletność (1-10) | 10 | 9 |
| Precyzja (1-10) | 9 | 9 |

### Obserwacje
- Różnica Haiku vs Sonnet: Sonnet znalazł serwery firmowe (Hummer, Atlas, Blade), backup OVH, Sokrates kontenery. Haiku ograniczył się do kluczowych plików VPS/hosting.
- agent-search vs klasyczne: A-Sonnet (agent-search) najkompletniejszy (16 plików) — BM25 "infrastruktura hosty kontenery" pokrył szeroki zakres tematów. Klasyczny Grep wymagał iterowania po wielu keywordach.

---

## Wyniki: T09 — Comparative analysis

> Porównaj podejście do nauczania Java/Spring vs Python/Django.

### Haiku

| Metryka | agent-search | klasyczne |
|---------|-------------|-----------|
| Wywołania narzędzi | 13 | 20 |
| Tokeny (k) | 65.1 | 70.4 |
| Czas (s) | 55.2 | 50.9 |
| Pliki znalezione | 6 | 15 |
| Kompletność (1-10) | 8 | 9 |
| Precyzja (1-10) | 8 | 8 |

### Sonnet

| Metryka | agent-search | klasyczne |
|---------|-------------|-----------|
| Wywołania narzędzi | 23 | 24 |
| Tokeny (k) | 81.7 | 88.7 |
| Czas (s) | 111.5 | 129.5 |
| Pliki znalezione | 10 | 18 |
| Kompletność (1-10) | 9 | 10 |
| Precyzja (1-10) | 9 | 9 |

### Obserwacje
- Różnica Haiku vs Sonnet: B-Haiku (klasyczny) zaskoczył — Glob po katalogach lekcji pozwoliło czytać porównawczo. B-Sonnet (klasyczny) najkompletniejszy, 18 plików.
- agent-search vs klasyczne: Klasyczne narzędzia lepsze — Glob + Read na dwóch katalogach (Spring/Django) to naturalna ścieżka. Agent-search nie dodał wartości przy zadaniu z dobrze znanymi ścieżkami.

---

## Wyniki: T10 — Full vault synthesis

> Jakie produkty edukacyjne istnieją lub były planowane? Nazwa, status, grupa, kanał, lejek.

### Haiku

| Metryka | agent-search | klasyczne |
|---------|-------------|-----------|
| Wywołania narzędzi | 46 | 23 |
| Tokeny (k) | 76.2 | 90.8 |
| Czas (s) | 112.7 | 64.5 |
| Pliki znalezione | 13 | 10 |
| Kompletność (1-10) | 8 | 6 |
| Precyzja (1-10) | 7 | 8 |

### Sonnet

| Metryka | agent-search | klasyczne |
|---------|-------------|-----------|
| Wywołania narzędzi | 55 | 151 |
| Tokeny (k) | 85.9 | 144.2 |
| Czas (s) | 222.0 | 457.3 |
| Pliki znalezione | 25 | 38 |
| Kompletność (1-10) | 9 | 10 |
| Precyzja (1-10) | 8 | 8 |

### Obserwacje
- Różnica Haiku vs Sonnet: Ogromna przepaść. B-Sonnet znalazł 20 produktów/38 plików ale kosztem 151 tool calls i 457s. A-Sonnet efektywniejszy: 15 produktów/25 plików w 222s.
- agent-search vs klasyczne: A-Haiku (agent-search) znalazł 11 produktów vs B-Haiku 7 — **agent-search pomaga słabszemu modelowi** na najtrudniejszym zadaniu. B-Sonnet (klasyczny) był bruteforce'owy ale najkompletniejszy.
- **Kluczowy insight**: Na zadaniu vault-wide synthesis, agent-search wyrównuje gap między Haiku a Sonnet (A-Haiku 8/7 vs B-Haiku 6/8).

---

## Podsumowanie zbiorcze

### Haiku

| Zadanie | Trudność | Wywołania A/K | Tokeny A/K | Czas A/K | Kompletność A/K | Precyzja A/K | Zwycięzca |
|---------|----------|---------------|------------|----------|-----------------|-------------|-----------|
| T01 | 1 | 4/2 | 24.5/22.4 | 11.2/6.3 | 8/8 | 10/10 | Remis (K szybszy) |
| T02 | 2 | 11/14 | 30.3/59.3 | 85.6/64.8 | 7/9 | 10/9 | Klasyczne |
| T03 | 3 | 19/20 | 62.9/37.5 | 55.0/44.3 | 7/8 | 9/9 | Klasyczne |
| T04 | 4 | 23/20 | 51.8/52.8 | 78.2/34.4 | 9/8 | 8/9 | agent-search (kompletność) |
| T05 | 5 | 13/11 | 92.4/79.6 | 43.4/29.0 | 9/8 | 8/9 | agent-search (kompletność) |
| T06 | 6 | 12/17 | 73.9/62.7 | 35.6/31.6 | 7/8 | 8/9 | Klasyczne |
| T07 | 7 | 24/20 | 49.7/42.1 | 53.5/32.7 | 8/7 | 9/9 | agent-search |
| T08 | 8 | 23/22 | 72.0/91.1 | 63.4/45.6 | 7/8 | 8/9 | Klasyczne |
| T09 | 9 | 13/20 | 65.1/70.4 | 55.2/50.9 | 8/9 | 8/8 | Klasyczne |
| T10 | 10 | 46/23 | 76.2/90.8 | 112.7/64.5 | 8/6 | 7/8 | **agent-search** |
| **Średnia** | | 18.8/16.9 | 59.9/60.9 | 59.4/40.4 | 7.8/7.9 | 8.5/8.9 | 4A / 5K / 1R |

### Sonnet

| Zadanie | Trudność | Wywołania A/K | Tokeny A/K | Czas A/K | Kompletność A/K | Precyzja A/K | Zwycięzca |
|---------|----------|---------------|------------|----------|-----------------|-------------|-----------|
| T01 | 1 | 4/3 | 13.1/10.8 | 19.6/11.1 | 10/10 | 10/10 | Remis (K szybszy) |
| T02 | 2 | 7/6 | 12.5/13.8 | 30.9/23.5 | 8/7 | 10/10 | agent-search |
| T03 | 3 | 25/31 | 41.3/47.9 | 92.9/88.5 | 9/10 | 9/9 | Klasyczne |
| T04 | 4 | 25/7 | 31.3/11.4 | 99.0/29.0 | 10/8 | 9/10 | agent-search (kompletność) |
| T05 | 5 | 27/28 | 82.4/92.8 | 108.7/111.8 | 9/10 | 9/9 | Klasyczne |
| T06 | 6 | 34/22 | 55.1/73.0 | 167.8/96.0 | 10/9 | 8/9 | **agent-search** |
| T07 | 7 | 14/13 | 27.7/22.8 | 83.8/69.7 | 10/9 | 10/10 | agent-search |
| T08 | 8 | 35/32 | 81.7/88.6 | 151.7/131.6 | 10/9 | 9/9 | agent-search |
| T09 | 9 | 23/24 | 81.7/88.7 | 111.5/129.5 | 9/10 | 9/9 | Klasyczne |
| T10 | 10 | 55/151 | 85.9/144.2 | 222.0/457.3 | 9/10 | 8/8 | Klasyczne (kompletność) |
| **Średnia** | | 24.9/31.7 | 51.3/59.4 | 108.8/114.8 | 9.4/9.2 | 9.1/9.3 | 5A / 4K / 1R |

### Porównanie modeli — czy narzędzie wyrównuje gap?

| Metryka | Haiku + agent-search | Haiku + klasyczne | Sonnet + agent-search | Sonnet + klasyczne |
|---------|---------------------|-------------------|----------------------|-------------------|
| Śr. wywołania | 18.8 | 16.9 | 24.9 | 31.7 |
| Śr. tokeny (k) | 59.9 | 60.9 | 51.3 | 59.4 |
| Śr. czas (s) | 59.4 | 40.4 | 108.8 | 114.8 |
| Śr. kompletność | 7.8 | 7.9 | 9.4 | 9.2 |
| Śr. precyzja | 8.5 | 8.9 | 9.1 | 9.3 |

### Kluczowe pytania do odpowiedzi

**1. Czy Haiku + agent-search dorównuje Sonnet + klasyczne?**

**Nie.** Kompletność Haiku+AS (7.8) jest znacząco niższa niż Sonnet+klasyczne (9.2). Narzędzie nie kompensuje różnicy w jakości modelu. Haiku popełnia więcej kroków bez poprawy wyniku, a na złożonych zadaniach (T08, T10) nie potrafi wyciągnąć z wyników tyle co Sonnet.

**2. Przy jakim poziomie trudności słabszy model zaczyna odpadać?**

Od **trudności 6-7** (T06-T07). Na T01-T05 Haiku radzi sobie dobrze z obu podejściami. Od T06 widoczna różnica w kompletności — Haiku nie odkrywa powiązanych materiałów tak skutecznie jak Sonnet.

**3. Który tryb (search/grep/hybrid) najczęściej wybierały poszczególne modele?**

- **Haiku + agent-search**: preferował `grep` (literal match) — prostsze, bezpieczniejsze. Rzadko korzystał z `search --grep` (hybrid).
- **Sonnet + agent-search**: mieszał `search` (BM25) z `grep`, częściej używał `--mode files` do orientacji, potem `--mode chunks`. Częściej korzystał z hybrid mode.

**4. Gdzie różnica w jakości jest największa — w efektywności (kroki/tokeny) czy w merytoryce?**

**W merytoryce.** Sonnet konsekwentnie znajduje więcej plików i buduje pełniejszy obraz tematu. Efektywność (tokeny/czas) jest zaskakująco wyrównana — Haiku nie jest dramatycznie szybszy, bo kompensuje słabszą trafność większą liczbą prób.

### Dodatkowe wnioski

1. **agent-search najlepiej sprawdza się na zadaniach hybrid (T06)** — łączenie semantyki z regex daje wyraźną przewagę nad iteracyjnym Grep.
2. **Na trivialnych zadaniach (T01-T02) agent-search jest overhead** — Grep/Glob wystarczają.
3. **Klasyczne narzędzia wygrywają gdy struktura plików jest znana** (T09 — porównanie dwóch katalogów).
4. **agent-search pomaga na vault-wide synthesis (T10)** — BM25 szybciej orientuje się w rozproszonych tematach niż iteracyjny Grep po keywordach.
5. **Sonnet zużywa mniej tokenów per task** mimo lepszych wyników — podejmuje trafniejsze decyzje o tym co przeczytać.
6. **B-Sonnet (klasyczny) na T10 był brutalnie drogi** — 151 tool calls, 144k tokenów, 457s — ale też najkompletniejszy (20 produktów).
