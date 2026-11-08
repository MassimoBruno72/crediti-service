# Crediti Service (versione installabile)

Registro incassi per Windows e Mac. Funziona senza Claude e senza internet.
I dati stanno in un file sul tuo computer, non nel programma né nel browser.

## Dove sono i dati

| Sistema | Cartella |
|---|---|
| Windows | `C:\Users\<nome>\AppData\Roaming\it.massimo.crediti-service\` |
| Mac | `~/Library/Application Support/it.massimo.crediti-service/` |

- `dati.json`: tutti i crediti e le quote mensili.
- `dati.backup-AAAA-MM-GG.json`: una copia al giorno, ne restano 14.
- `dati.prima-importazione-…json`: creata prima di ogni importazione, non viene mai cancellata.

Gli aggiornamenti sostituiscono il programma e non toccano questa cartella.

## Primo uso

1. Installa l'app (file `.exe`/`.msi` per Windows, `.dmg` per Mac, dalla pagina Release).
2. Apri **Dati e aggiornamenti → Importa dati…** e scegli `dati-iniziali.json`
   (i 31 crediti e la quota di ottobre presi dall'app su Claude).

Primo avvio: Windows mostra l'avviso SmartScreen («Maggiori informazioni → Esegui comunque»).
Su Mac usa **Impostazioni di Sistema → Privacy e sicurezza → Apri comunque**.
Succede perché l'app non ha una firma a pagamento. Compare solo al primo avvio.

## Aggiornamenti

Dal pannello **Dati e aggiornamenti**:

- **Cerca aggiornamenti**: scarica e installa l'ultima versione (serve internet).
- **Installa una patch da file**: sceglie un file `.crpatch` scaricato dalla pagina Release
  (anche da un altro dispositivo). La patch viene accettata solo se la firma è valida.

## Come si pubblica una modifica (lo fa Claude)

1. Si modifica `src/index.html`.
2. Si aumenta la versione in `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml` e `package.json`.
3. Commit, poi tag: `git tag v1.0.1 && git push --tags`.
4. GitHub compila gli installer Windows e Mac, li firma per gli aggiornamenti e allega
   anche il file `.crpatch`. Nessun passaggio manuale.

Una patch cambia solo la pagina (interfaccia e logica). Se cambia il codice Rust
(`src-tauri/`) serve invece un aggiornamento completo.

## Configurazione una tantum

1. Crea su GitHub un repository **pubblico** chiamato `crediti-service` (contiene solo il codice, mai i dati).
2. In `src-tauri/tauri.conf.json` sostituisci `UTENTE` con il tuo nome utente GitHub.
3. Nel repository: **Settings → Secrets and variables → Actions → New repository secret**:
   - `TAURI_SIGNING_PRIVATE_KEY` = contenuto del file `crediti.key`
   - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` = lascia vuoto (la chiave non ha password)
4. Carica il codice e crea la prima tag `v1.0.0`.

Il file `crediti.key` è la chiave che autorizza gli aggiornamenti: non metterla nel repository
e conservane una copia al sicuro. Se la perdi, le app già installate non accetteranno più
aggiornamenti e andranno reinstallate.

## Prove in locale

```
npm install
npm run dev      # richiede Rust e le librerie di Tauri
```
