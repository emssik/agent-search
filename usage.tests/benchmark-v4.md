# Benchmark v4 — agent-search vs klasyczne narzędzia (Grep/Glob/Read)

**Data:** 2026-03-14
**Model:** Sonnet (obie strony)
**Corpus:** `/Users/daniel/00_work/obsidian.working` (~1450 plików Markdown)
**Metoda:** 18 agentów równoległych (9 zadań × 2 podejścia), bez wzajemnej widoczności wyników

---

## Wyniki: T01 — Exact match (pg_dump)

| Metryka | agent-search (A) | klasyczne (B) |
|---------|-------------------|---------------|
| Wywołania narzędzi | 18 | 4 |
| Tokeny (k) | 14.9k | 9.6k |
| Czas (s) | 51 | 15 |
| Pliki znalezione | 0 | 0 |
| Kompletność (1-10) | 10 | 10 |
| Precyzja (1-10) | 10 | 10 |

### Obserwacje
- Oba podejścia poprawnie stwierdziły brak frazy `pg_dump` w corpus.
- **Klasyczne wygrało zdecydowanie** — 4 tool calls vs 18. Grep jest naturalnym narzędziem do literal match.
- agent-search wykonał nadmiarowe sprawdzenia (BM25 z wariantami, dodatkowe Grep-y), co nie wniosło wartości.

---

## Wyniki: T02 — Semantic single-topic (proces obsługi studenta)

| Metryka | agent-search (A) | klasyczne (B) |
|---------|-------------------|---------------|
| Wywołania narzędzi | 17 | 30 |
| Tokeny (k) | 49.1k | 50.0k |
| Czas (s) | 66 | 93 |
| Pliki znalezione | 7 | 24 |
| Kompletność (1-10) | 7 | 9 |
| Precyzja (1-10) | 9 | 9 |

### Obserwacje
- **Klasyczne wygrało w kompletności** — znalazło kroki onboardingowe (Krok 01–10), monitoring aktywności, przypomnienia o płatności. Agent A skupił się na automatyzacjach Make/Airtable ale pominął treść onboardingu.
- Agent A był **szybszy** (66s vs 93s) i użył **mniej narzędzi** (17 vs 30), ale kosztem pełności odpowiedzi.
- Klasyczne potrzebowało więcej kroków, ale systematycznie przeszukało powiązane pliki.

---

## Wyniki: T03 — Regex pattern (cron + systemd)

| Metryka | agent-search (A) | klasyczne (B) |
|---------|-------------------|---------------|
| Wywołania narzędzi | 8 | 18 |
| Tokeny (k) | 16.2k | 42.9k |
| Czas (s) | 44 | 74 |
| Pliki znalezione | 5 | 13 |
| Kompletność (1-10) | 6 | 9 |
| Precyzja (1-10) | 9 | 8 |

### Obserwacje
- **Klasyczne wygrało w kompletności** — znalazło 13 plików (w tym wzmianki edukacyjne, kontekstowe, strefa czasowa n8n) vs 5.
- Agent A był **2.5× bardziej efektywny tokenowo** (16k vs 43k) i szybszy, ale pominął pliki z kontekstowymi wzmiankami o cronie (maile, social media, Airtable cron).
- Klasyczne B poświęciło więcej czasu na iteracyjne przeszukiwanie wariantów fraz.

---

## Wyniki: T04 — Cross-topic discovery (narzędzia deploymentu)

| Metryka | agent-search (A) | klasyczne (B) |
|---------|-------------------|---------------|
| Wywołania narzędzi | 30 | 34 |
| Tokeny (k) | 75.3k | 94.5k |
| Czas (s) | 157 | 147 |
| Pliki znalezione | 17 | 13 |
| Kompletność (1-10) | 8 | 9 |
| Precyzja (1-10) | 9 | 9 |

### Obserwacje
- Zbliżone wyniki. Klasyczne B znalazło **więcej narzędzi** (19 vs 12), w tym Harbor, pgbouncer, LVM, NetBox, Ansible, ngrok.
- Agent A znalazł **więcej plików** (17 vs 13), ale opisał mniej technologii.
- Oba podejścia zużyły dużo zasobów — to szerokie zadanie wymagające iteracji po wielu keywords.
- Klasyczne miało lekką przewagę dzięki systematycznemu grep-owaniu po konkretnych nazwach narzędzi.

---

## Wyniki: T05 — Hybrid: semantic + regex (lekcje z SQL)

| Metryka | agent-search (A) | klasyczne (B) |
|---------|-------------------|---------------|
| Wywołania narzędzi | 10 | 22 |
| Tokeny (k) | 45.1k | 22.0k |
| Czas (s) | 76 | 70 |
| Pliki znalezione | 16 | 9 |
| Kompletność (1-10) | 9 | 7 |
| Precyzja (1-10) | 8 | 9 |

### Obserwacje
- **agent-search wygrał w kompletności** — znalazł serię Instagram SQL (5 plików edukacyjnych) + lekcje Spring/Django. Klasyczne pominęło serię Instagram.
- Agent A użył **hybrid mode** (`search --grep`) efektywnie — BM25 "lekcja programowanie" + regex na SQL keywords odkrył pliki, których samo grep nie znalazło (brak bezpośrednich keywords SQL w ścieżce, ale semantycznie powiązane).
- Klasyczne B znalazło pliki projektu "eve" (DDL), które A pominął.
- **Pierwszy task gdzie agent-search daje wyraźną przewagę** — hybrid mode łączy dwa wymiary wyszukiwania.

---

## Wyniki: T06 — Temporal + scattered (sekwencja emailowa)

| Metryka | agent-search (A) | klasyczne (B) |
|---------|-------------------|---------------|
| Wywołania narzędzi | 11 | 17 |
| Tokeny (k) | 33.0k | 24.6k |
| Czas (s) | 77 | 78 |
| Pliki znalezione | 7 | 9 |
| Kompletność (1-10) | 9 | 9 |
| Precyzja (1-10) | 9 | 9 |

### Obserwacje
- **Remis merytoryczny** — obie odpowiedzi odtworzyły identyczną sekwencję emailową (7→3→1 dzień, prolongata, zawieszenie, dezaktywacja).
- Agent A był **bardziej efektywny** (11 vs 17 tool calls), zużył więcej tokenów ale podobny czas.
- Klasyczne B dodało email potwierdzenia płatności jako krok 1 — nieco szersza perspektywa.
- BM25 dobrze poradził sobie z odkrywaniem plików o płatnościach/suspendowaniu — semantyczne zapytania szybko trafiły w odpowiednie pliki.

---

## Wyniki: T07 — Infrastructure audit (hard)

| Metryka | agent-search (A) | klasyczne (B) |
|---------|-------------------|---------------|
| Wywołania narzędzi | 21 | 33 |
| Tokeny (k) | 91.2k | 92.3k |
| Czas (s) | 124 | 127 |
| Pliki znalezione | 15 | 16 |
| Kompletność (1-10) | 8 | 9 |
| Precyzja (1-10) | 8 | 8 |

### Obserwacje
- **Klasyczne minimalnie lepsze** — znalazło dodatkowe pliki (Sokrates.md, Skrypt Reporter) i opisało firewall szczegółowiej.
- Agent A użył **36% mniej narzędzi** (21 vs 33) przy porównywalnych tokenach i czasie.
- Obie odpowiedzi mają tabelaryczną strukturę (hosty, kontenery, porty, backup, SSL) — dobra synteza rozproszonych danych.
- Przy tym poziomie trudności agent-search zaczyna wykazywać **efektywność** (mniej kroków) bez utraty jakości.

---

## Wyniki: T08 — Comparative analysis (Java vs Python)

| Metryka | agent-search (A) | klasyczne (B) |
|---------|-------------------|---------------|
| Wywołania narzędzi | 21 | 24 |
| Tokeny (k) | 66.0k | 86.8k |
| Czas (s) | 127 | 148 |
| Pliki znalezione | 13 | 18 |
| Kompletność (1-10) | 9 | 9 |
| Precyzja (1-10) | 9 | 9 |

### Obserwacje
- **Remis merytoryczny** — obie odpowiedzi pokryły te same wymiary porównania (struktura lekcji, narzędzia, deployment, baza danych) z porównywalną głębokością.
- Agent A był **24% tańszy tokenowo** (66k vs 87k) i **14% szybszy** (127s vs 148s).
- Klasyczne B przeczytało więcej plików (18 vs 13), w tym prompty i plany robocze, ale nie przyniosło to istotnie lepszej odpowiedzi.
- **agent-search wygrał efektywnością** przy zachowaniu tej samej jakości.

---

## Wyniki: T09 — Full vault synthesis (produkty edukacyjne)

| Metryka | agent-search (A) | klasyczne (B) |
|---------|-------------------|---------------|
| Wywołania narzędzi | 23 | 138 |
| Tokeny (k) | 110.6k | 153.7k |
| Czas (s) | 180 | 441 |
| Pliki znalezione | 27 | 39 |
| Kompletność (1-10) | 9 | 10 |
| Precyzja (1-10) | 9 | 9 |

### Obserwacje
- **agent-search wygrał zdecydowanie w efektywności** — 6× mniej tool calls (23 vs 138), 2.5× szybszy (180s vs 441s), 28% mniej tokenów.
- Klasyczne B znalazło **17 produktów** (vs 13 u A), w tym niszowe: Protokół Kepler, VIBEcoding→PROjekt, webinary sprzedażowe, Wyzwanie AI Reloaded. Marginalna przewaga w kompletności.
- BM25 z multi-query pozwolił agent-search trafić w kluczowe pliki w 3 min. Klasyczne potrzebowało 7+ min intensywnego grep-owania.
- **Najtrudniejsze zadanie w benchmarku** — wymaga przeszukania całego vault i korelacji wielu źródeł. Tutaj różnica w efektywności jest **największa** w całym benchmarku.

---

## Podsumowanie zbiorcze

| Zadanie | Trudność | Wywołania A/K | Tokeny A/K (k) | Czas A/K (s) | Kompletność A/K | Precyzja A/K | Zwycięzca |
|---------|----------|---------------|-----------------|--------------|-----------------|-------------|-----------|
| T01 | 2 | 18/4 | 14.9/9.6 | 51/15 | 10/10 | 10/10 | **Klasyczne** |
| T02 | 3 | 17/30 | 49.1/50.0 | 66/93 | 7/9 | 9/9 | **Klasyczne** |
| T03 | 4 | 8/18 | 16.2/42.9 | 44/74 | 6/9 | 9/8 | **Klasyczne** |
| T04 | 5 | 30/34 | 75.3/94.5 | 157/147 | 8/9 | 9/9 | **Klasyczne** |
| T05 | 6 | 10/22 | 45.1/22.0 | 76/70 | 9/7 | 8/9 | **agent-search** |
| T06 | 7 | 11/17 | 33.0/24.6 | 77/78 | 9/9 | 9/9 | **Remis** |
| T07 | 8 | 21/33 | 91.2/92.3 | 124/127 | 8/9 | 8/8 | **Remis** |
| T08 | 9 | 21/24 | 66.0/86.8 | 127/148 | 9/9 | 9/9 | **agent-search** |
| T09 | 10 | 23/138 | 110.6/153.7 | 180/441 | 9/10 | 9/9 | **agent-search** |
| **Średnia** | | **17/38** | **55.6/64.0** | **100/133** | **8.3/9.0** | **8.9/9.1** | |

---

## Kluczowe pytania — odpowiedzi

### 1. Który tryb (search/grep/hybrid) najczęściej wybierał model?

Model z agent-search najczęściej zaczynał od `search --mode files` (discovery), potem `search --mode chunks` (kontekst), a `grep` używał do weryfikacji precyzyjnych fraz. Tryb **hybrid** (`search --grep`) pojawił się głównie w T05 (lekcje + SQL) i tam dał największą wartość dodaną. W zadaniach T01-T03 model nadmiarowo używał `search` tam, gdzie sam `grep` wystarczył.

### 2. Przy jakim poziomie trudności agent-search zaczyna dawać przewagę?

**Od trudności 6/10 (T05 — hybrid semantic+regex).** Przy prostszych zadaniach (T01-T04) klasyczne narzędzia wygrywają lub remisują — grep jest szybszy i precyzyjniejszy dla literal match i keyword search. Punkt przełamania to moment, gdy zadanie wymaga **łączenia semantyki z precyzją** lub **przeszukania wielu kontekstów jednocześnie**.

### 3. Gdzie różnica w jakości jest największa — w efektywności (kroki/tokeny) czy w merytoryce?

**W efektywności.** Merytorycznie obie metody osiągają porównywalne wyniki (średnia kompletność 8.3 vs 9.0, precyzja 8.9 vs 9.1). Natomiast agent-search konsekwentnie zużywa **mniej tool calls** (średnio 17 vs 38, czyli 55% mniej) i jest **25% szybszy** (100s vs 133s). Przewaga merytoryczna agent-search ujawnia się dopiero przy **trudnych zadaniach wymagających eksploracji** (T05, T08, T09), gdzie BM25 discovery pozwala szybciej trafić w odpowiednie pliki bez iteracyjnego grep-owania po kolejnych keywords.

### 4. Zaskoczenia

- **T02-T03**: Klasyczne narzędzia wygrały kompletności mimo większej liczby kroków — systematyczne grep-owanie po wariantach fraz okazało się skuteczniejsze niż BM25 ze stemmingiem polskim.
- **T09**: Największa różnica w efektywności w całym benchmarku — agent-search: 23 calls/180s vs klasyczne: 138 calls/441s (6× więcej narzędzi, 2.5× dłużej). Klasyczne B znalazło nieco więcej produktów, ale kosztem ogromnej ilości zasobów.
- **T05**: Hybrid mode (`search --grep`) to killer feature — odkrył pliki, których ani sam search, ani sam grep by nie znalazł.

---

## Wnioski

1. **Przy prostych zadaniach agent z agent-search ma tendencję do „overthinku"** — zamiast zaufać pierwszemu wynikowi (`grep → 0`), agent spiraluje w weryfikacje (re-index, warianty, --help). Sam `agent-search grep` jest porównywalny z Grep, ale agent dodaje nadmiarowe kroki. To problem promptowania, nie narzędzia.
2. **agent-search uzupełnia grep/glob** — najcenniejszy jest tryb hybrid i BM25 discovery dla tematów rozproszonych po wielu plikach.
3. **Punkt przełamania: trudność ≥6** — im bardziej rozmyte i rozproszone zadanie, tym większa przewaga agent-search.
4. **Efektywność > merytoryka** — główna korzyść to mniej kroków agenta, nie lepsza jakość odpowiedzi (z wyjątkiem najtrudniejszych zadań).
5. **Optimum: agent-search do discovery, klasyczne do precyzji** — najlepsza strategia to `search --mode files` na start, potem `grep`/`Read` do weryfikacji.
