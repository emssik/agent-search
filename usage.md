# agent-search — usage guide

## Kiedy użyć którego trybu

| Potrzeba | Komenda | Dlaczego |
|----------|---------|----------|
| Dokładny ciąg/regex (nazwa funkcji, error code) | `grep` | Nie potrzebujesz indeksu, dopasowanie regex po liniach |
| Semantyczne/rozmyte wyszukiwanie ("jak działa auth") | `search` | BM25 ze stemmingiem |
| Precyzyjne wyszukiwanie w temacie | `search --grep` | BM25 zawęża temat, regex filtruje wynik końcowy |

## Quick reference

```bash
# 1. Indeksowanie (wymagane przed search, nie przed grep)
agent-search index -c ./projekt

# 2. Indeksowanie z konkretnym stemmerem
agent-search index -c ./projekt --language en

# 3. BM25 search — eksploracja tematu
agent-search search -c ./projekt -q "obsługa błędów"

# 4. Regex grep — precyzyjne dopasowanie
agent-search grep -c ./projekt -p "TODO|FIXME|HACK"

# 5. Hybrid — BM25 + regex
agent-search search -c ./projekt -q "authentication" --grep "JWT"
```

## Indeks i język stemmera (`index --language`)

Obsługiwane wartości: `pl`, `en`, `de`, `fr`, `es`, `it`, `pt`, `ru`, `sv`, `nl`, `fi`, `da`, `hu`, `ro`, `tr`, `none`.

```bash
# Polski (domyślny dla nowego indeksu)
agent-search index -c . --language pl

# Angielski
agent-search index -c . --language en

# Bez stemmingu
agent-search index -c . --language none
```

Zasady:
- Gdy tworzysz nowy indeks i nie podasz `--language`, użyty będzie `pl`.
- Gdy indeks już istnieje i nie podasz `--language`, zachowany zostanie język z manifestu.
- Gdy podasz `--language` inny niż zapisany w indeksie, narzędzie automatycznie przebuduje indeks.

## Tryby wyjścia (`--mode`)

**chunks** (domyślny) — fragmenty kodu z kontekstem.

```bash
agent-search search -c ./projekt -q "error handling"
agent-search grep -c ./projekt -p "panic!" --context-lines 5
```

**files** — tylko ścieżki + score.

```bash
agent-search search -c ./projekt -q "database" --mode files
agent-search grep -c ./projekt -p "CREATE TABLE" --mode files
```

**summary** — wyniki pogrupowane po katalogach.

```bash
agent-search search -c ./projekt -q "test" --mode summary
agent-search grep -c ./projekt -p "(?i)auth" --mode summary
```

## Filtrowanie ścieżek (`--include`, `--exclude`)

Działa w `search` i `grep`. Wzorce to globy (`globset`).

```bash
# Tylko markdowni w docs
agent-search search -c . -q "migration" --include "docs/**/*.md"

# Wyklucz legacy i vendor
agent-search grep -c . -p "TODO" --exclude "legacy/**" --exclude "vendor/**"

# Include + exclude razem
agent-search search -c . -q "auth" --include "src/**" --exclude "src/generated/**"
```

## Sortowanie (`--sort`)

Działa dla `--mode files` i `--mode summary` w `search` i `grep`.

Opcje:
- `score` (domyślnie)
- `path` (alfabetycznie)
- `mtime` (najnowsze pliki/katalogi najpierw)

```bash
agent-search search -c . -q "database" --mode files --sort path
agent-search grep -c . -p "error" --mode summary --sort mtime
```

## Optymalne wzorce użycia

### Rozpoznanie terenu
```bash
# Gdzie w kodzie żyje temat X?
agent-search search -c . -q "authentication" --mode summary

# Które pliki dotyczą tematu?
agent-search search -c . -q "payment processing" --mode files --max-results 10
```

### Precyzyjne wyszukiwanie
```bash
# Exact string match
agent-search grep -c . -p "fn validate_token"

# Regex z alternatywami
agent-search grep -c . -p "rsync|rclone|restic"

# Case-insensitive
agent-search grep -c . -p "(?i)fixme"
```

### Hybrid: zawężenie + filtrowanie
```bash
# BM25 znajduje kontekst backupu, regex zostawia tylko rsync
agent-search search -c . -q "backup strategy" --grep "^rsync"

# BM25 + regex w trybie files
agent-search search -c . -q "configuration" --grep "\.env|dotenv" --mode files
```

### Multi-query
```bash
# Szukaj wielu terminów naraz, wyniki mergowane
agent-search search -c . -q "authenticate" -q "authorize" --mode files
```

### Kontrola rozmiaru wyjścia
```bash
# Więcej kontekstu per chunk
agent-search search -c . -q "error" --context-lines 20

# Większy budżet tokenów (dla dużych kontekstów LLM)
agent-search search -c . -q "migration" --token-budget 16000

# Mniej wyników
agent-search grep -c . -p "SELECT.*FROM" --max-results 5
```

## Domyślne wartości

| Flaga | search | grep |
|-------|--------|------|
| `--context-lines` | 10 | 2 |
| `--token-budget` | 4096 | 4096 |
| `--max-results` | 100 | 100 |
| `--mode` | chunks | chunks |
| `--sort` | score | score |

`index`:
- `--language`: brak flagi => nowy indeks `pl`, istniejący indeks => język z manifestu

## Scoring

- **search** — BM25 score z Tantivy, z boostem x3 na ścieżkę pliku i density scoring per chunk
- **grep** — score = liczba matching lines w pliku (files mode) lub match density w chunku (chunks mode)
- **hybrid** (`--grep`) — BM25 score zachowany, regex tylko filtruje (nie zmienia score)

## Dobre praktyki

1. Zacznij od `--mode files`, potem przełącz na `chunks`.
2. Grep nie wymaga indeksu, więc nadaje się do szybkiego ad-hoc.
3. Hybrid oszczędza tokeny: BM25 zawęża temat, regex odcina szum.
4. Multi-query (`-q` wiele razy) zwykle działa lepiej niż jedno długie zapytanie.
5. `--context-lines 0` w grep daje minimalne chunki z samymi liniami dopasowania.
