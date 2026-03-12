Opierając się na badaniach naukowych nad architekturą **GrepRAG** oraz mechanizmach wbudowanych w zaawansowane narzędzia takie jak **Context Mode**, oto konkretny algorytm bezwektorowego wyszukiwania agentowego (Agentic/Lexical Search), idealnie nadający się do obsługi ticketów z poziomu systemu plików:

Algorytm ten składa się z **5 głównych etapów**, gdzie szybkie narzędzia systemowe wykonują ciężką pracę, a lekki skrypt w Pythonie (post-processing) filtruje szum przed wysłaniem ostatecznych danych do modelu LLM.

### Krok 1: Generowanie zapytań (Query Generation)
Gdy do systemu trafia nowe zapytanie od użytkownika (nowy ticket), Agent AI (LLM) nie przeszukuje od razu wszystkiego. Najpierw analizuje lokalny kontekst problemu i **autonomicznie generuje zestaw precyzyjnych komend** (np. `m = 10` równoległych komend dla narzędzia `ripgrep` lub zapytań do bazy SQLite FTS5).
*   Agent używa identyfikatorów z ticketu: np. numerów błędów, nazw modułów czy logów serwera.
*   Wykorzystuje zaawansowane techniki, takie jak używanie "dzikich kart" (wildcards) do wyszukiwania wzorców (np. `rg "błąd_logowania_.*"`), co podnosi elastyczność zapytań.

### Krok 2: Surowa egzekucja leksykalna (Deterministic Execution)
Zestaw wygenerowanych komend jest uruchamiany w systemie plików (np. przez `ripgrep`). Narzędzie to działa wielowątkowo i natychmiast filtruje niechciane pliki binarne czy foldery uwzględnione w `.gitignore`.
*   **Fuzzy Search (Wyszukiwanie rozmyte):** Jeśli w architekturze używamy silnika takiego jak SQLite FTS5, w tym kroku nakładane są 3 warstwy elastyczności: klasyczne szukanie rdzeni słów (stemming), algorytm trigramów na wyłapanie części ciągów znaków (np. "autentyk" łapie "autentykacja") oraz korekta literówek z użyciem odległości Levenshteina.
*   Zamiast przesyłać całe pliki, wyciągane są określone bloki linii (np. używając flagi `--context` w `ripgrep`), aby otoczyć znalezione słowo kluczowe odpowiednim kontekstem semantycznym.
*   Wynikiem tego etapu jest tzw. surowa pula kandydatów (Raw Chunks).

### Krok 3: Ważony Re-ranking algorytmem BM25 (Identifier-Weighted Re-ranking)
Jednym z głównych problemów czystego `grep` jest to, że jeśli agent wyszuka ogólne słowa (jak `init`, `config` czy `run`), system wyciągnie masę bezużytecznych wyników, czyli tzw. szum informacyjny.
*   Aby to naprawić, wszystkie zebrane w Kroku 2 fragmenty tekstu (kandydaci) są oceniane i szeregowane na nowo algorytmem **BM25** (zamiast np. naiwnego podobieństwa Jaccarda).
*   BM25 automatycznie nakłada karę na bardzo pospolite terminy, a drastycznie podnosi wagę dla unikalnych identyfikatorów związanych z konkretnym ticketem. Otrzymujemy posortowaną listę, z której do dalszej analizy wybiera się przeważnie tylko górne 50% najlepszych dopasowań (odrzucając resztę jako szum).

### Krok 4: Świadoma struktury deduplikacja i fuzja (Structure-Aware De-duplication)
Niezależnie od siebie działające wyszukiwania agenta mogą wielokrotnie trafić w te same miejsca, marnując cenne miejsce w oknie kontekstowym LLM na duplikaty lub zaburzając logiczny ciąg czytania tekstu.
*   Algorytm analizuje fizyczne numery linii i ścieżki plików z wyselekcjonowanej w Kroku 3 puli fragmentów.
*   Jeśli różne fragmenty (chunks) nachodzą na siebie lub sąsiadują w obrębie tego samego pliku (np. jeden chunk to linie 1-10, a drugi 8-15), **algorytm skleja je w jedną, płynną całość** (linie 1-15). 
*   Eliminuje to fragmentację tekstu, chroniąc model przed czytaniem instrukcji, w których rozwiązanie znajduje się np. przed opisem.

### Krok 5: Inteligentne przycinanie i odpowiedź (Smart Snippets & Generation)
Na samym końcu, po odsianiu słabych wyników i sklejeniu duplikatów, ostateczna lista fragmentów trafia pod kontrolę limitów:
*   Pula wyselekcjonowanych i połączonych bloków jest ucinana w ten sposób, by zmieścić się w sztywnym budżecie tokenów systemu (np. Top-K limitowane do dokładnie 4096 tokenów).
*   Taki spakowany, maksymalnie wyczyszczony z szumu kontekst ("Session Guide") jest ostatecznie przekazywany do głównego okna modelu LLM, by ten wygenerował finalną propozycję odpowiedzi na rozwiązywany ticket.

Dzięki temu post-processingowi, agentowe wyszukiwanie leksykalne działa w zaledwie ułamki sekund (zamiast sekund/minut jak ma to miejsce w bazach grafowych lub wektorowych), jednocześnie osiągając wyniki wielokrotnie przewyższające naiwne podawanie modelowi wszystkiego co wypluje komenda wyszukiwania.