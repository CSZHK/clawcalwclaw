# Αναφορά Εντολών clawclawclaw (CLI Reference)

Αυτός ο οδηγός περιλαμβάνει το πλήρες σύνολο των εντολών που είναι διαθέσιμες στη διεπαφή γραμμής εντολών (CLI) του clawclawclaw.

Τελευταία ενημέρωση: 3 Μαρτίου 2026.

## Σύνοψη Εντολών

| Εντολή | Περιγραφή |
|:---|:---|
| `onboard` | Εκκίνηση της διαδικασίας αρχικής διαμόρφωσης και εγγραφής. |
| `agent` | Έναρξη αλληλεπίδρασης με τον πράκτορα AI (Interactive Mode). |
| `tui` | Εκκίνηση full-screen terminal UI (απαιτεί feature `tui-ratatui`). |
| `daemon` | Εκτέλεση του clawclawclaw ως διεργασία παρασκηνίου (Background Process). |
| `service` | Διαχείριση της υπηρεσίας συστήματος (System Service). |
| `doctor` | Εκτέλεση διαγνωστικών ελέγχων ακεραιότητας και συνδεσιμότητας. |
| `status` | Προβολή της τρέχουσας κατάστασης και των ενεργών ρυθμίσεων. |
| `cron` | Διαχείριση προγραμματισμένων εργασιών και αυτοματισμών. |
| `models` | Συγχρονισμός και διαχείριση διαθέσιμων μοντέλων AI. |
| `providers` | Διαχείριση των παρόχων υπολογιστικής ισχύος (LLM Providers). |
| `channel` | Διαμόρφωση και έλεγχος των καναλιών επικοινωνίας. |
| `skills` | Διαχείριση των επεκτάσεων και δυνατοτήτων (Skills) του πράκτορα. |
| `hardware` | Ανίχνευση και διαχείριση συνδεδεμένου υλικού (USB/Serial). |

---

## Ανάλυση Κύριων Εντολών

### 1. `onboard` (Αρχική Διαμόρφωση)

- `clawclawclaw onboard --interactive`: Διαδραστική καθοδήγηση για τη ρύθμιση του συστήματος.
- `clawclawclaw onboard --channels-only`: Εστιασμένη διαμόρφωση αποκλειστικά για τα κανάλια επικοινωνίας.

### 2. `agent` (Διαδραστική Λειτουργία)

- `clawclawclaw agent`: Έναρξη τυπικής συνομιλίας.
- `clawclawclaw agent -m "<μήνυμα>"`: Άμεση αποστολή εντολής/μηνύματος στον πράκτορα.

> [!TIP]
> Κατά τη διάρκεια της συνομιλίας, μπορείτε να αιτηθείτε την αλλαγή του μοντέλου (π.χ. "use gpt-4") και ο πράκτορας θα προσαρμόσει τις ρυθμίσεις του δυναμικά.

### 2.0 `tui` (Τερματικό UI)

- `clawclawclaw tui`
- `clawclawclaw tui --provider <ID> --model <MODEL>`

Σημειώσεις:
- Απαιτεί build με `--features tui-ratatui`.
- Χωρίς το feature, η εντολή επιστρέφει friendly μήνυμα για rebuild.
- Βασικά shortcuts:
  - `Enter`: αποστολή μηνύματος (editing mode)
  - `Shift+Enter`: νέα γραμμή
  - `Ctrl+C`: ακύρωση του τρέχοντος in-flight request
  - διπλό `Ctrl+C` μέσα σε 300ms: force quit
  - `q` ή `Ctrl+D`: έξοδος

### 2.1 `gateway` / `daemon`

- `clawclawclaw gateway [--host <HOST>] [--port <PORT>] [--new-pairing]`
- `clawclawclaw daemon [--host <HOST>] [--port <PORT>]`
- Το `--new-pairing` καθαρίζει όλα τα αποθηκευμένα paired tokens και δημιουργεί νέο pairing code κατά την εκκίνηση του gateway.

### 2.2 OpenClaw Migration Surface

- `clawclawclaw onboard --migrate-openclaw`
- `clawclawclaw onboard --migrate-openclaw --openclaw-source <PATH> --openclaw-config <PATH>`
- `clawclawclaw migrate openclaw --dry-run`
- `clawclawclaw migrate openclaw`

Σημείωση: στο agent runtime υπάρχει επίσης το εργαλείο `openclaw_migration` για controlled preview/apply migration flows.

### 3. `cron` (Προγραμματισμός Εργασιών)

Δυνατότητα αυτοματισμού εντολών:
- `clawclawclaw cron add "0 9 * * *" "echo Daily Setup"`: Εκτέλεση καθημερινά στις 09:00.
- `clawclawclaw cron once "1h" "clawclawclaw status"`: Προγραμματισμός εκτέλεσης μετά από μία ώρα.

### 4. `doctor` (Διάγνωση Συστήματος)

Χρησιμοποιήστε την εντολή `clawclawclaw doctor` για την επαλήθευση της ορθής λειτουργίας των εξαρτήσεων, της πρόσβασης στο διαδίκτυο και της εγκυρότητας του αρχείου ρυθμίσεων.

### 5. `skills` (Επεκτασιμότητα)

- `clawclawclaw skills list`: Προβολή εγκατεστημένων δεξιοτήτων.
- `clawclawclaw skills install <source>`: Εγκατάσταση νέας δεξιότητας από εξωτερική πηγή.

> [!NOTE]
> Το clawclawclaw εφαρμόζει αυτόματη ανάλυση κώδικα (security scanning) σε κάθε νέα δεξιότητα πριν την ενεργοποίησή της για την αποφυγή εκτέλεσης κακόβουλου λογισμικού.

---

## Βοήθεια και Τεκμηρίωση

Για αναλυτικές πληροφορίες σχετικά με τις παραμέτρους κάθε εντολής, χρησιμοποιήστε το flag `--help`:
`clawclawclaw <command> --help`
(π.χ. `clawclawclaw onboard --help`)
